import type { ReactNode } from "react";

const P: Record<string, ReactNode> = {
  min: <path d="M4 8h8" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" />,
  max: <rect x="4" y="4" width="8" height="8" rx="1" fill="none" stroke="currentColor" strokeWidth="1.25" />,
  close: <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" />,
  search: (
    <>
      <circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <path d="M10.5 10.5 14 14" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </>
  ),
  ext: <path d="M6 4h6v6M12 4l-7.5 7.5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />,
  check: <path d="m4 8.5 2.5 2.5L12 5.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />,
  down: <path d="M4.5 6.5 8 10l3.5-3.5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />,
  refresh: <path d="M13 8a5 5 0 1 1-1.5-3.55M13 2.5v3h-3" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />,
  zap: <path d="M9 1.5 3.5 9H8l-1 5.5L12.5 7H8z" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round" />,
  lock: (
    <>
      <rect x="3.5" y="7" width="9" height="6.5" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <path d="M5.5 7V5.5a2.5 2.5 0 0 1 5 0V7" fill="none" stroke="currentColor" strokeWidth="1.5" />
    </>
  ),
  cookie: (
    <>
      <path d="M13.6 8.6A5.8 5.8 0 1 1 7.4 2.4a2.2 2.2 0 0 0 2.7 2.7 2.2 2.2 0 0 0 3.5 3.5Z" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round" />
      <circle cx="5.6" cy="7" r=".9" fill="currentColor" />
      <circle cx="8.6" cy="10.6" r=".9" fill="currentColor" />
      <circle cx="5.4" cy="10.4" r=".7" fill="currentColor" />
    </>
  ),
  history: (
    <>
      <circle cx="8" cy="8" r="5.8" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <path d="M8 4.8V8l2.2 1.6" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </>
  ),
  download: <path d="M8 2.5v7.5M4.8 7 8 10.2 11.2 7M3 13.5h10" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />,
  storage: <path d="M3 4.5c0-1.1 2.2-2 5-2s5 .9 5 2v7c0 1.1-2.2 2-5 2s-5-.9-5-2zM3 4.5c0 1.1 2.2 2 5 2s5-.9 5-2M3 8c0 1.1 2.2 2 5 2s5-.9 5-2" fill="none" stroke="currentColor" strokeWidth="1.4" />,
  layers: <path d="M8 2 2 5.2l6 3.2 6-3.2Z M2 8.2l6 3.2 6-3.2M2 11l6 3.2 6-3.2" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round" />,
  all: (
    <>
      <rect x="2" y="9" width="5" height="5" rx="1.2" fill="currentColor" />
      <rect x="9" y="5.5" width="3.5" height="3.5" rx=".8" fill="currentColor" opacity=".8" />
      <rect x="12" y="2" width="2.5" height="2.5" rx=".6" fill="currentColor" opacity=".6" />
    </>
  ),
  cache: (
    <>
      <rect x="2.5" y="3" width="11" height="10" rx="2" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <path d="M2.5 6.5h11M5 4.8h.01M7 4.8h.01" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </>
  ),
  form: (
    <>
      <rect x="2.5" y="3" width="11" height="4" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <rect x="2.5" y="9" width="11" height="4" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <path d="M5 5h3" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </>
  ),
  pin: (
    <>
      <path d="M8 14s4.5-4.2 4.5-7.5a4.5 4.5 0 0 0-9 0C3.5 9.8 8 14 8 14Z" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <circle cx="8" cy="6.5" r="1.6" fill="currentColor" />
    </>
  ),
};

export function Icon({ name, className = "ic" }: { name: string; className?: string }) {
  return (
    <svg className={className} viewBox="0 0 16 16" aria-hidden="true">
      {P[name]}
    </svg>
  );
}

const BROWSER_GLYPH: Record<string, ReactNode> = {
  chrome: <path fill="currentColor" d="M12 0C8.21 0 4.831 1.757 2.632 4.501l3.953 6.848A5.454 5.454 0 0 1 12 6.545h10.691A12 12 0 0 0 12 0zM1.931 5.47A11.943 11.943 0 0 0 0 12c0 6.012 4.42 10.991 10.189 11.864l3.953-6.847a5.45 5.45 0 0 1-6.865-2.29zm13.342 2.166a5.446 5.446 0 0 1 1.45 7.09l.002.001h-.002l-5.344 9.257c.206.01.413.016.621.016 6.627 0 12-5.373 12-12 0-1.54-.29-3.011-.818-4.364zM12 16.364a4.364 4.364 0 1 1 0-8.728 4.364 4.364 0 0 1 0 8.728Z" />,
  firefox: <path fill="currentColor" d="M22.778 8.048c-.505-1.215-1.53-2.528-2.333-2.943.654 1.283 1.033 2.57 1.177 3.53l.002.02c-1.314-3.278-3.544-4.6-5.366-7.477-.091-.147-.184-.292-.273-.446a3.545 3.545 0 01-.13-.24 2.118 2.118 0 01-.172-.46.03.03 0 00-.027-.03.038.038 0 00-.021 0l-.006.001a.037.037 0 00-.01.005L15.624 0c-2.585 1.515-3.657 4.168-3.932 5.856a6.197 6.197 0 00-2.305.587.297.297 0 00-.147.37c.057.162.24.24.396.17a5.622 5.622 0 012.008-.523l.067-.005a5.847 5.847 0 011.957.222l.095.03a5.816 5.816 0 01.616.228c.08.036.16.073.238.112l.107.055a5.835 5.835 0 01.368.211 5.953 5.953 0 012.034 2.104c-.62-.437-1.733-.868-2.803-.681 4.183 2.09 3.06 9.292-2.737 9.02a5.164 5.164 0 01-1.513-.292 4.42 4.42 0 01-.538-.232c-1.42-.735-2.593-2.121-2.74-3.806 0 0 .537-2 3.845-2 .357 0 1.38-.998 1.398-1.287-.005-.095-2.029-.9-2.817-1.677-.422-.416-.622-.616-.8-.767a3.47 3.47 0 00-.301-.227 5.388 5.388 0 01-.032-2.842c-1.195.544-2.124 1.403-2.8 2.163h-.006c-.46-.584-.428-2.51-.402-2.913-.006-.025-.343.176-.389.206-.406.29-.787.616-1.136.974-.397.403-.76.839-1.085 1.303a9.816 9.816 0 00-1.562 3.52c-.003.013-.11.487-.19 1.073-.013.09-.026.181-.037.272a7.8 7.8 0 00-.069.667l-.002.034-.023.387-.001.06C.386 18.795 5.593 24 12.016 24c5.752 0 10.527-4.176 11.463-9.661.02-.149.035-.298.052-.448.232-1.994-.025-4.09-.753-5.844z" />,
};

export function BrowserGlyph({ id, className = "g" }: { id: string; className?: string }) {
  const glyph = BROWSER_GLYPH[id];
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      {glyph ?? (
        <>
          <circle cx="12" cy="12" r="10.5" fill="none" stroke="currentColor" strokeWidth="2" />
          <text x="12" y="16.5" textAnchor="middle" fontSize="12" fontWeight="700" fill="currentColor" fontFamily="inherit">
            {id[0].toUpperCase()}
          </text>
        </>
      )}
    </svg>
  );
}

export function Mark({ className = "mk" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path d="M5 6a5 5 0 0 0-5 5v8a5 5 0 0 0 5 5h8a5 5 0 0 0 5-5v-1h-6v-6H6V6Z" fill="#5B4BE8" />
      <path d="M20 1v3h3V1ZM10 8h5V3h-5ZM16 14h5V9h-5Z" fill="#FF6B5A" />
    </svg>
  );
}

export function Kbd({ k }: { k: string }) {
  return (
    <span className="kbd" aria-hidden="true">
      {k}
    </span>
  );
}
