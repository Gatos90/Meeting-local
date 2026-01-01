'use client'

import { useState, useEffect } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import {
  Home,
  FileText,
  Calendar,
  Settings,
  ChevronLeft,
  ChevronRight,
  Mic,
  ClipboardList,
  Wrench,
  Cpu,
  HardDrive,
  Monitor,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { getHardwareRecommendations, HardwareRecommendations } from '@/lib/hardware'

const navigation = [
  { name: 'Home', href: '/', icon: Home },
  { name: 'Transcripts', href: '/transcripts', icon: FileText },
  { name: 'Templates', href: '/templates', icon: ClipboardList },
  { name: 'Tools (in development)', href: '/tools', icon: Wrench },
  { name: 'Schedule (coming soon)', href: '/schedule', icon: Calendar },
  { name: 'Settings', href: '/settings', icon: Settings },
]

export function AppSidebar() {
  const [collapsed, setCollapsed] = useState(false)
  const [hardwareInfo, setHardwareInfo] = useState<HardwareRecommendations | null>(null)
  const pathname = usePathname()

  useEffect(() => {
    getHardwareRecommendations()
      .then(setHardwareInfo)
      .catch(console.error)
  }, [])

  return (
    <TooltipProvider delayDuration={0}>
      <aside
        className={cn(
          "flex flex-col border-r border-border bg-sidebar transition-all duration-300",
          collapsed ? "w-16" : "w-64"
        )}
      >
        {/* Logo */}
        <div className="flex h-16 items-center border-b border-border px-4">
          <div className="flex items-center gap-2 text-primary">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10">
              <Mic className="h-5 w-5" />
            </div>
            {!collapsed && (
              <span className="text-lg font-bold tracking-tight text-foreground">
                MeetLocal
              </span>
            )}
          </div>
        </div>

        {/* Navigation */}
        <nav className="flex-1 space-y-1 p-3">
          {navigation.map((item) => {
            const isActive = pathname === item.href
            const Icon = item.icon

            if (collapsed) {
              return (
                <Tooltip key={item.name}>
                  <TooltipTrigger asChild>
                    <Link
                      href={item.href}
                      className={cn(
                        "flex h-10 w-10 items-center justify-center rounded-lg transition-colors",
                        isActive
                          ? "bg-sidebar-accent text-sidebar-primary"
                          : "text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
                      )}
                    >
                      <Icon className="h-5 w-5" />
                    </Link>
                  </TooltipTrigger>
                  <TooltipContent side="right">
                    {item.name}
                  </TooltipContent>
                </Tooltip>
              )
            }

            return (
              <Link
                key={item.name}
                href={item.href}
                className={cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                  isActive
                    ? "bg-sidebar-accent text-sidebar-primary"
                    : "text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
                )}
              >
                <Icon className={cn(
                  "h-5 w-5",
                  isActive ? "text-sidebar-primary" : "text-muted-foreground"
                )} />
                <span>{item.name}</span>
                {isActive && (
                  <div className="ml-auto h-1.5 w-1.5 rounded-full bg-sidebar-primary" />
                )}
              </Link>
            )
          })}
        </nav>

        <Separator />

        {/* Hardware Info - Compact with values */}
        {!collapsed && hardwareInfo && (
          <div className="px-3 py-2 flex items-center justify-center gap-4 text-xs text-muted-foreground">
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-1 cursor-default">
                  <Cpu className="h-3.5 w-3.5" />
                  <span>{hardwareInfo.hardware.cpu_cores}</span>
                </div>
              </TooltipTrigger>
              <TooltipContent side="top">
                {hardwareInfo.hardware.cpu_cores} CPU Cores
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-1 cursor-default">
                  <HardDrive className="h-3.5 w-3.5" />
                  <span>{hardwareInfo.hardware.memory_gb}GB</span>
                </div>
              </TooltipTrigger>
              <TooltipContent side="top">
                {hardwareInfo.hardware.memory_gb} GB System Memory
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-1 cursor-default">
                  <Monitor className="h-3.5 w-3.5" />
                  <span>{hardwareInfo.hardware.has_gpu ? 'GPU' : 'CPU'}</span>
                </div>
              </TooltipTrigger>
              <TooltipContent side="top">
                {hardwareInfo.hardware.has_gpu ? hardwareInfo.hardware.gpu_type : 'CPU Only (No GPU detected)'}
              </TooltipContent>
            </Tooltip>
          </div>
        )}

        {/* Collapse Toggle */}
        <div className="p-3">
          <Button
            variant="ghost"
            size="icon"
            className="w-full justify-center"
            onClick={() => setCollapsed(!collapsed)}
          >
            {collapsed ? (
              <ChevronRight className="h-5 w-5" />
            ) : (
              <ChevronLeft className="h-5 w-5" />
            )}
          </Button>
        </div>
      </aside>
    </TooltipProvider>
  )
}
