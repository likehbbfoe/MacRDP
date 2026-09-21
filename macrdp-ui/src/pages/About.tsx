import { useState, type MouseEvent } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { Monitor } from "lucide-react";
import { version } from "../../package.json";
import { api } from "../lib/ipc";

function About() {
  const [linkError, setLinkError] = useState(false);

  const openPage = async (
    event: MouseEvent<HTMLAnchorElement>,
    page: "repository" | "releases"
  ) => {
    if (!isTauri()) return;
    event.preventDefault();
    setLinkError(false);
    try {
      await api.openProjectPage(page);
    } catch {
      setLinkError(true);
    }
  };

  return (
    <div className="flex h-full flex-col items-center justify-center p-4">
      <div className="w-full max-w-sm rounded-[10px] bg-card p-6 text-center">
        <div className="mx-auto mb-3 flex h-16 w-16 items-center justify-center rounded-2xl bg-accent/10">
          <Monitor size={32} className="text-accent" />
        </div>

        <div className="text-lg font-semibold text-text">MacRDP</div>
        <div className="mt-0.5 text-xs text-text-muted">版本 {version}</div>

        <div className="my-3 border-t border-border" />

        <div className="text-[11px] text-text-muted">
          IronRDP &middot; OpenH264 &middot; ScreenCaptureKit
        </div>

        <div className="mt-3 flex items-center justify-center gap-3">
          <a
            href="https://github.com/likehbbfoe/MacRDP"
            onClick={(event) => openPage(event, "repository")}
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs text-accent hover:underline"
          >
            GitHub
          </a>
          <span className="text-xs text-text-muted">GPLv3 License</span>
          <a
            href="https://github.com/likehbbfoe/MacRDP/releases/latest"
            onClick={(event) => openPage(event, "releases")}
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs text-accent hover:underline"
          >
            发行说明
          </a>
        </div>
        {linkError && (
          <p role="alert" className="mt-2 text-xs text-text-muted">
            无法打开浏览器，请右键复制链接后访问。
          </p>
        )}
      </div>
    </div>
  );
}

export default About;
