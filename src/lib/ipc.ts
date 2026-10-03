import { invoke } from "@tauri-apps/api/core";

export type Profile = { id: string; name: string; avatar: string | null };
export type Browser = { id: string; name: string; family: "chromium" | "firefox"; process: string; profiles: Profile[] };
export type Site = {
  domain: string;
  cookies: number;
  historyUrls: number;
  visits: number;
  downloads: number;
  storageBytes: number;
  siteCacheBytes: number;
  lastVisit: number | null;
  lastCookieAccess: number | null;
};
export type FormField = { name: string; entries: number; lastUsed: number | null };
export type Scan = { sites: Site[]; cacheBytes: number; forms: FormField[]; addresses: number; locked: string[]; fromShadow: boolean };

export const listBrowsers = () => invoke<Browser[]>("list_browsers");
export const scanProfile = (browserId: string, profileId: string) => invoke<Scan>("scan_profile", { browserId, profileId });
export const runningProcesses = () => invoke<string[]>("running_processes");
export const siteIcons = (browserId: string, profileId: string) => invoke<Record<string, string>>("site_icons", { browserId, profileId });

export type CleanRequest = { sites: Record<string, string[]>; profile: string[] };
export type CleanReport = { freedBytes: number; backup: string | null };
export type CleanError = { message: string; touched: boolean };
export const cleanProfile = (browserId: string, profileId: string, request: CleanRequest, close: boolean) =>
  invoke<CleanReport>("clean_profile", { browserId, profileId, request, close });
