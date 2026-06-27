export type AppStatus =
  | "not_installed"
  | "installed"
  | "update_available"
  | "unsupported"; // no installable asset for this platform

export interface AppEntry {
  /** repo name, e.g. "oxiterm" */
  name: string;
  /** human display name, e.g. "Oxiterm" */
  display_name: string;
  description: string;
  /** owner/name */
  full_name: string;
  html_url: string;
  /** latest release tag, e.g. "v0.7.1" (null if no release) */
  latest_version: string | null;
  /** locally installed version, null if not installed */
  installed_version: string | null;
  status: AppStatus;
  /** download url of the asset for the current platform */
  asset_url: string | null;
  asset_name: string | null;
  /** ISO date of latest release */
  released_at: string | null;
  /** true if this repo appeared after the last refresh */
  is_new: boolean;
}

export interface Settings {
  username: string;
  auto_check_minutes: number;
}

export type Progress = {
  app: string;
  phase: "download" | "install" | "done" | "error";
  message: string;
  /** 0..100, -1 = indeterminate */
  percent: number;
};
