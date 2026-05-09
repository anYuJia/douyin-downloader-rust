import { useAppStore, useDownloadStore } from "@/stores/app-store";
import type { ViewType } from "@/types";
import { Badge } from "@/components/ui/badge";
import {
  Home,
  Search,
  Link2,
  Sparkles,
  FolderOpen,
  Heart,
  Settings,
  Circle,
  Star,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { cn } from "@/lib/utils";

interface NavItem {
  id: ViewType;
  label: string;
  icon: React.ElementType;
  command?: "search" | "link";
}

const navItems: NavItem[] = [
  { id: "home", label: "首页", icon: Home },
  { id: "search", label: "搜索用户", icon: Search, command: "search" },
  { id: "link", label: "粘贴链接", icon: Link2, command: "link" },
  { id: "recommended", label: "推荐视频", icon: Sparkles },
  { id: "downloads", label: "我的下载", icon: FolderOpen },
  { id: "liked", label: "点赞视频", icon: Heart },
  { id: "collected", label: "收藏内容", icon: Star },
  { id: "settings", label: "设置", icon: Settings },
];

const SIDEBAR_EXPANDED_WIDTH = 220;
const SIDEBAR_COLLAPSED_WIDTH = 56;

export function Sidebar() {
  const currentView = useAppStore((s) => s.currentView);
  const setView = useAppStore((s) => s.setView);
  const setCommandOpen = useAppStore((s) => s.setCommandOpen);
  const setCommandMode = useAppStore((s) => s.setCommandMode);
  const sidebarCollapsed = useAppStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);
  const cookieLoggedIn = useAppStore((s) => s.cookieLoggedIn);
  const activeCount = useDownloadStore((s) => s.activeCount);

  const handleNavClick = (item: NavItem) => {
    if (item.command) {
      setCommandMode(item.command);
      setCommandOpen(true);
    } else {
      setView(item.id);
    }
  };

  const width = sidebarCollapsed ? SIDEBAR_COLLAPSED_WIDTH : SIDEBAR_EXPANDED_WIDTH;

  return (
    <motion.aside
      className="flex flex-col h-full border-r border-border bg-gradient-to-b from-surface-solid/94 to-background-soft/86 shrink-0 overflow-hidden"
      animate={{ width }}
      transition={{ type: "spring", stiffness: 400, damping: 30 }}
    >
      {/* Brand */}
      <div className="flex items-center gap-3 px-4 py-4 shrink-0">
        <div className="w-9 h-9 rounded-[12px] overflow-hidden flex items-center justify-center shrink-0">
          <img src="/animated_icon.svg" alt="Douyin Downloader" className="w-9 h-9" />
        </div>
        <AnimatePresence mode="wait">
          {!sidebarCollapsed && (
            <motion.div
              className="flex flex-col min-w-0"
              initial={{ opacity: 0, x: -4 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -4 }}
              transition={{ duration: 0.15 }}
            >
              <span className="text-[0.82rem] font-[780] tracking-tight text-text truncate">
                DY Downloader
              </span>
              <span className="text-[0.62rem] font-semibold text-text-muted tracking-wide">
                本地媒体工作台
              </span>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Collapse toggle */}
      <div className="flex justify-end px-3 mb-1 shrink-0">
        <button
          onClick={toggleSidebar}
          className="flex items-center justify-center w-7 h-7 rounded-[8px] text-text-muted hover:text-text hover:bg-surface-raised transition-colors cursor-pointer"
          title={sidebarCollapsed ? "展开导航" : "收起导航"}
        >
          {sidebarCollapsed ? (
            <ChevronRight className="w-3.5 h-3.5" />
          ) : (
            <ChevronLeft className="w-3.5 h-3.5" />
          )}
        </button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 flex flex-col gap-0.5 px-2 overflow-y-auto">
        {!sidebarCollapsed && (
          <div className="text-[0.62rem] font-bold text-text-muted uppercase tracking-[0.08em] px-2 mb-1.5 mt-1">
            导航
          </div>
        )}

        {navItems.map((item) => {
          const isActive = currentView === item.id;
          const Icon = item.icon;

          return (
            <motion.button
              key={item.label}
              onClick={() => handleNavClick(item)}
              className={cn(
                "group relative flex items-center h-[38px] rounded-[12px] text-left transition-[background-color,color,box-shadow,transform] duration-[var(--duration-fast)] ease-[var(--ease-spring)] cursor-pointer",
                sidebarCollapsed ? "justify-center px-0" : "gap-2.5 px-2.5",
                isActive
                  ? "bg-accent-soft text-accent shadow-[inset_0_0_0_1px_var(--color-accent-ring)]"
                  : "text-text-muted hover:text-text hover:bg-surface-raised"
              )}
              whileHover={{ scale: 1.01 }}
              whileTap={{ scale: 0.98 }}
              title={sidebarCollapsed ? item.label : undefined}
            >
              {isActive && (
                <motion.div
                  layoutId="sidebar-active"
                  className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-4 rounded-r-full bg-accent"
                  transition={{ type: "spring", stiffness: 400, damping: 30 }}
                />
              )}

              <Icon className={cn("w-[18px] h-[18px] shrink-0", sidebarCollapsed && "mx-auto")} />

              <AnimatePresence mode="wait">
                {!sidebarCollapsed && (
                  <motion.span
                    className="text-[0.78rem] font-semibold truncate"
                    initial={{ opacity: 0, x: -4 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: -4 }}
                    transition={{ duration: 0.15 }}
                  >
                    {item.label}
                  </motion.span>
                )}
              </AnimatePresence>

              {item.id === "downloads" && activeCount > 0 && !sidebarCollapsed && (
                <Badge variant="default" size="sm" className="ml-auto">
                  {activeCount}
                </Badge>
              )}
              {item.id === "downloads" && activeCount > 0 && sidebarCollapsed && (
                <span className="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-accent" />
              )}
            </motion.button>
          );
        })}
      </nav>

      {/* Status — pinned to bottom */}
      <div className={cn("px-2 py-2 shrink-0", sidebarCollapsed && "flex justify-center")}>
        <div className={cn(
          "flex items-center gap-2 rounded-[12px] bg-surface/50 text-text-muted",
          sidebarCollapsed ? "justify-center w-9 h-9" : "px-3 h-[38px]"
        )}>
          <Circle className={cn(
            "w-2 h-2 shrink-0",
            cookieLoggedIn ? "fill-success text-success" : "fill-warning text-warning"
          )} />
          {!sidebarCollapsed && (
            <span className="text-[0.68rem] font-medium">
              {cookieLoggedIn ? "已登录" : "需要登录 Cookie"}
            </span>
          )}
        </div>
      </div>
    </motion.aside>
  );
}
