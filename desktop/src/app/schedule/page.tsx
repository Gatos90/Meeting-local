'use client'

import { Calendar, Clock, Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'

// Mock scheduled recordings
const scheduledRecordings = [
  {
    id: '1',
    title: 'Weekly Team Sync',
    time: 'Tomorrow, 9:00 AM',
    duration: '30 min',
    recurring: true,
  },
  {
    id: '2',
    title: 'Client Demo',
    time: 'Thu, Dec 5, 2:00 PM',
    duration: '1 hour',
    recurring: false,
  },
]

export default function SchedulePage() {
  return (
    <>
      {/* Header */}
      <header className="flex h-16 items-center justify-between border-b border-border bg-background/50 backdrop-blur-sm px-8">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold text-foreground">Schedule</h1>
          <Badge variant="secondary">Coming Soon</Badge>
        </div>

        <Button disabled className="gap-2">
          <Plus className="h-4 w-4" />
          Schedule Recording
        </Button>
      </header>

      {/* Main Content */}
      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-6xl space-y-8">
          {/* Feature Preview */}
          <div className="flex flex-col items-center justify-center py-12 text-center space-y-4">
            <div className="p-4 rounded-full bg-muted mb-4">
              <Calendar className="h-8 w-8 text-muted-foreground" />
            </div>
            <h2 className="text-3xl font-bold tracking-tight text-foreground">
              Scheduled Recordings
            </h2>
            <p className="text-muted-foreground max-w-lg">
              Automatically start recordings at scheduled times. Perfect for
              recurring meetings, interviews, and planned sessions.
            </p>
          </div>

          {/* Upcoming Recordings Preview */}
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-foreground">
              Upcoming (Preview)
            </h3>
            <div className="space-y-3">
              {scheduledRecordings.map((recording) => (
                <Card
                  key={recording.id}
                  className="p-4 opacity-60 cursor-not-allowed"
                >
                  <div className="flex items-center justify-between">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <h4 className="font-medium text-foreground">
                          {recording.title}
                        </h4>
                        {recording.recurring && (
                          <Badge variant="outline" className="text-xs">
                            Recurring
                          </Badge>
                        )}
                      </div>
                      <div className="flex items-center gap-4 text-sm text-muted-foreground">
                        <span className="flex items-center gap-1">
                          <Calendar className="h-3 w-3" />
                          {recording.time}
                        </span>
                        <span className="flex items-center gap-1">
                          <Clock className="h-3 w-3" />
                          {recording.duration}
                        </span>
                      </div>
                    </div>
                    <Button variant="outline" size="sm" disabled>
                      Edit
                    </Button>
                  </div>
                </Card>
              ))}
            </div>
          </div>

          {/* Feature Roadmap */}
          <Card className="p-6 bg-muted/30">
            <h3 className="font-medium text-foreground mb-4">
              Planned Features
            </h3>
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li className="flex items-center gap-2">
                <span className="h-1.5 w-1.5 rounded-full bg-primary" />
                One-time and recurring schedules
              </li>
              <li className="flex items-center gap-2">
                <span className="h-1.5 w-1.5 rounded-full bg-primary" />
                Calendar integration (Google, Outlook)
              </li>
              <li className="flex items-center gap-2">
                <span className="h-1.5 w-1.5 rounded-full bg-primary" />
                Automatic device selection per schedule
              </li>
              <li className="flex items-center gap-2">
                <span className="h-1.5 w-1.5 rounded-full bg-primary" />
                Pre-meeting notifications
              </li>
            </ul>
          </Card>
        </div>
      </div>
    </>
  )
}
