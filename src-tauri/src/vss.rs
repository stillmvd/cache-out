use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf, Prefix};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Shadow {
    id: String,
    device: String,
    volume: String,
}

fn powershell(script: &str) -> io::Result<String> {
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(io::Error::other(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn journal() -> PathBuf {
    std::env::temp_dir().join("cache-out").join("shadows.txt")
}

fn remember(id: &str, keep: bool) {
    let path = journal();
    let mut ids: Vec<String> = fs::read_to_string(&path).unwrap_or_default().lines().filter(|l| *l != id && !l.is_empty()).map(String::from).collect();
    if keep {
        ids.push(id.to_string());
    }
    let _ = fs::create_dir_all(path.parent().unwrap());
    let _ = fs::write(&path, ids.join("\n"));
}

fn remove(id: &str) -> io::Result<()> {
    powershell(&format!("Get-CimInstance Win32_ShadowCopy | Where-Object ID -eq '{id}' | Remove-CimInstance"))?;
    remember(id, false);
    Ok(())
}

pub fn sweep_stale() {
    for id in fs::read_to_string(journal()).unwrap_or_default().lines().filter(|l| !l.is_empty()) {
        let _ = remove(id);
    }
}

pub fn volume_of(path: &Path) -> Option<String> {
    match path.components().next()? {
        Component::Prefix(p) => match p.kind() {
            Prefix::Disk(d) | Prefix::VerbatimDisk(d) => Some(format!("{}:\\", d as char)),
            _ => None,
        },
        _ => None,
    }
}

pub fn shadow_path(device: &str, original: &Path) -> Option<PathBuf> {
    let rest: PathBuf = original.components().skip_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir)).collect();
    Some(PathBuf::from(format!("{}\\{}", device.trim_end_matches('\\'), rest.display())))
}

impl Shadow {
    pub fn create(volume: &str) -> io::Result<Self> {
        let out = powershell(&format!(
            "$r = Invoke-CimMethod -ClassName Win32_ShadowCopy -MethodName Create -Arguments @{{Volume='{volume}'; Context='ClientAccessible'}}; \
             if ($r.ReturnValue -ne 0) {{ [Console]::Error.WriteLine(\"vss $($r.ReturnValue)\"); exit 1 }}; \
             $s = Get-CimInstance Win32_ShadowCopy | Where-Object ID -eq $r.ShadowID; \
             Write-Output \"$($s.ID)|$($s.DeviceObject)\""
        ))?;
        let (id, device) = out.lines().last().and_then(|l| l.split_once('|')).ok_or_else(|| io::Error::other(format!("vss: {out}")))?;
        remember(id, true);
        Ok(Self { id: id.into(), device: device.into(), volume: volume.into() })
    }

    pub fn covers(&self, original: &Path) -> bool {
        volume_of(original).is_some_and(|v| v.eq_ignore_ascii_case(&self.volume))
    }

    pub fn path(&self, original: &Path) -> Option<PathBuf> {
        shadow_path(&self.device, original)
    }
}

impl Drop for Shadow {
    fn drop(&mut self) {
        let _ = remove(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_paths_into_shadow_device() {
        let p = Path::new(r"C:\Users\me\AppData\Local\Google\Chrome\User Data\Default\Network\Cookies");
        assert_eq!(volume_of(p).as_deref(), Some("C:\\"));
        assert_eq!(
            shadow_path(r"\\?\GLOBALROOT\Device\HarddiskVolumeShadowCopy7", p).unwrap(),
            PathBuf::from(r"\\?\GLOBALROOT\Device\HarddiskVolumeShadowCopy7\Users\me\AppData\Local\Google\Chrome\User Data\Default\Network\Cookies")
        );
        assert_eq!(volume_of(Path::new("relative")), None);
    }
}
