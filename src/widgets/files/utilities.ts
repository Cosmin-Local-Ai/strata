// Quick Utilities: one click to the Windows tools you reach for most.
import { Activity, Binary, Calculator, Cpu, FileCode, HardDrive, Info, Keyboard, ListTree, NotebookPen, Scissors, Settings2, SquareTerminal, Terminal, TriangleAlert, Wrench } from '@lucide/svelte'

export const UTILITIES: { label: string; icon: any; target: string; args?: string }[] = [
  { label: 'Terminal', icon: SquareTerminal, target: 'wt.exe' },
  { label: 'PowerShell', icon: Terminal, target: 'powershell.exe' },
  { label: 'Command Prompt', icon: Binary, target: 'cmd.exe' },
  { label: 'Snipping Tool', icon: Scissors, target: 'ms-screenclip:' },
  { label: 'Task Manager', icon: Activity, target: 'taskmgr.exe' },
  { label: 'Calculator', icon: Calculator, target: 'calc.exe' },
  { label: 'Notepad', icon: NotebookPen, target: 'notepad.exe' },
  { label: 'Control Panel', icon: Settings2, target: 'control.exe' },
  { label: 'Device Manager', icon: Cpu, target: 'devmgmt.msc' },
  { label: 'Disk Management', icon: HardDrive, target: 'diskmgmt.msc' },
  { label: 'Services', icon: ListTree, target: 'services.msc' },
  { label: 'Registry Editor', icon: FileCode, target: 'regedit.exe' },
  { label: 'Event Viewer', icon: TriangleAlert, target: 'eventvwr.msc' },
  { label: 'Resource Monitor', icon: Wrench, target: 'resmon.exe' },
  { label: 'System Info', icon: Info, target: 'msinfo32.exe' },
  { label: 'Character Map', icon: Keyboard, target: 'charmap.exe' },
]
