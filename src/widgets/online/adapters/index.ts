import type { Adapter } from '../../../lib/services.svelte'
import { AI } from './ai'
import { DEV } from './dev'
import { FINANCE } from './finance'
import { MARKETS, SOCIAL } from './social'
import { CLOUD } from './cloud'

export const ADAPTERS: Record<string, Adapter> = { ...AI, ...FINANCE, ...DEV, ...SOCIAL, ...MARKETS, ...CLOUD }
