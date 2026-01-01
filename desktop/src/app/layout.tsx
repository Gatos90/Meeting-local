import type { Metadata } from 'next'
import './globals.css'
import { AppSidebar } from '@/components/app-sidebar'

export const metadata: Metadata = {
  title: 'Meeting Local',
  description: 'Local meeting recording and transcription',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body className="overflow-hidden">
        <div className="flex h-screen w-full bg-background">
          <AppSidebar />
          <main className="flex-1 flex flex-col overflow-hidden">
            {children}
          </main>
        </div>
      </body>
    </html>
  )
}
