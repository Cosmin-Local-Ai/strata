// Microsoft Office and Google Workspace launchers: shared data.
import { Calendar, FileSpreadsheet, FileText, FolderOpen, HardDrive, Image, Mail, MessageSquare, NotebookPen, Presentation, SquareCheck, Video } from '@lucide/svelte'
import type { Component } from 'svelte'

export type Doc = { path: string; name: string }
export type GApp = { id: string; name: string; url: string; icon: Component<any>; color: string; create?: string }

export const GOOGLE: GApp[] = [
  { id: 'docs', name: 'Docs', url: 'https://docs.google.com/document/', icon: FileText, color: '#4285f4', create: 'https://docs.new' },
  { id: 'sheets', name: 'Sheets', url: 'https://docs.google.com/spreadsheets/', icon: FileSpreadsheet, color: '#0f9d58', create: 'https://sheets.new' },
  { id: 'slides', name: 'Slides', url: 'https://docs.google.com/presentation/', icon: Presentation, color: '#f4b400', create: 'https://slides.new' },
  { id: 'forms', name: 'Forms', url: 'https://docs.google.com/forms/', icon: SquareCheck, color: '#7248b9', create: 'https://forms.new' },
  { id: 'drive', name: 'Drive', url: 'https://drive.google.com/drive/home', icon: HardDrive, color: '#1fa463' },
  { id: 'gmail', name: 'Gmail', url: 'https://mail.google.com/', icon: Mail, color: '#ea4335' },
  { id: 'calendar', name: 'Calendar', url: 'https://calendar.google.com/', icon: Calendar, color: '#4285f4', create: 'https://cal.new' },
  { id: 'meet', name: 'Meet', url: 'https://meet.google.com/', icon: Video, color: '#00897b', create: 'https://meet.new' },
  { id: 'keep', name: 'Keep', url: 'https://keep.google.com/', icon: NotebookPen, color: '#fbbc04', create: 'https://keep.new' },
  { id: 'chat', name: 'Chat', url: 'https://chat.google.com/', icon: MessageSquare, color: '#34a853' },
  { id: 'photos', name: 'Photos', url: 'https://photos.google.com/', icon: Image, color: '#ea4335' },
]
export const GOOGLE_DEFAULT = ['docs', 'sheets', 'slides', 'drive', 'gmail', 'calendar']
export const OFFICE_DEFAULT = ['word', 'excel', 'powerpoint', 'outlook', 'outlook-new', 'onenote', 'teams']
export const OFFICE_EXT = /\.(docx?|docm|dotx|rtf|xlsx|xlsm|xlsb|xls|csv|pptx?|ppsx|one|vsdx|mpp|accdb|pub)$/i
export const OFFICE_FILTER: [string, string][] = [['Office documents', '*.docx;*.doc;*.docm;*.rtf;*.xlsx;*.xlsm;*.xls;*.csv;*.pptx;*.ppt;*.one;*.vsdx;*.accdb;*.pub;*.pdf'], ['All files', '*.*']]
export { FolderOpen }

/** Accent colour of an Office document by extension (Word blue, Excel green, PowerPoint orange…). */
export function docColor(name: string) {
  const e = name.toLowerCase().split('.').pop() ?? ''
  if (/^(docx?|docm|dotx|rtf)$/.test(e)) return '#2b7cd3'
  if (/^(xlsx|xlsm|xlsb|xls|csv)$/.test(e)) return '#21a366'
  if (/^(pptx?|ppsx)$/.test(e)) return '#d35230'
  if (e === 'one') return '#7719aa'
  if (e === 'pdf') return '#e5252a'
  return '#8e8e93'
}
