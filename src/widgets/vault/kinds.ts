// Vault item types: only the fields each kind needs. Each field is stored either in the index (no secrets: title, username, url) or
// as a secret field in Credential Manager / the encrypted store.
import { CreditCard, FileLock2, IdCard, KeyRound, Lock, StickyNote, Wifi } from '@lucide/svelte'
import type { Component } from 'svelte'

export type Kind = 'password' | 'api' | 'note' | 'card' | 'wifi' | 'identity' | 'file'
export type Field = {
  key: string
  label: string
  /** 'username' | 'url' live in the index (searchable, shown in the list); 'secret' / 'notes' are
   *  the main secret and notes; anything else is a secret field. */
  secret?: boolean
  multiline?: boolean
  placeholder?: string
  mono?: boolean
}
export type KindDef = { id: Kind; name: string; plural: string; icon: Component<any>; fields: Field[]; copyMain?: string }

export const KINDS: KindDef[] = [
  { id: 'password', name: 'Login', plural: 'Logins', icon: Lock, copyMain: 'Password', fields: [
    { key: 'username', label: 'Username or email' },
    { key: 'secret', label: 'Password', secret: true, mono: true },
  ] },
  { id: 'api', name: 'API key', plural: 'API keys', icon: KeyRound, copyMain: 'Key', fields: [
    { key: 'username', label: 'Service' },
    { key: 'secret', label: 'Key', secret: true, mono: true },
  ] },
  { id: 'note', name: 'Secure note', plural: 'Notes', icon: StickyNote, fields: [
    { key: 'notes', label: 'Note', multiline: true },
  ] },
  { id: 'card', name: 'Card', plural: 'Cards', icon: CreditCard, copyMain: 'Card number', fields: [
    { key: 'holder', label: 'Name on card', secret: true },
    { key: 'secret', label: 'Card number', secret: true, mono: true, placeholder: '1234 5678 9012 3456' },
    { key: 'expiry', label: 'Expiry (MM/YY)', secret: true, mono: true },
    { key: 'cvv', label: 'Security code (CVV)', secret: true, mono: true },
    { key: 'pin', label: 'PIN', secret: true, mono: true },
    { key: 'notes', label: 'Notes', multiline: true },
  ] },
  { id: 'wifi', name: 'Wi-Fi', plural: 'Wi-Fi', icon: Wifi, copyMain: 'Wi-Fi password', fields: [
    { key: 'username', label: 'Network name (SSID)' },
    { key: 'secret', label: 'Password', secret: true, mono: true },
  ] },
  { id: 'identity', name: 'ID document', plural: 'IDs', icon: IdCard, copyMain: 'Number', fields: [
    { key: 'username', label: 'Type (passport, ID card, driving licence)' },
    { key: 'fullName', label: 'Full name', secret: true },
    { key: 'secret', label: 'Document number', secret: true, mono: true },
    { key: 'personal', label: 'Personal number (CNP, SSN)', secret: true, mono: true },
    { key: 'issued', label: 'Issued', secret: true },
    { key: 'expires', label: 'Expires', secret: true },
    { key: 'country', label: 'Country', secret: true },
    { key: 'notes', label: 'Notes', multiline: true },
  ] },
  { id: 'file', name: 'File', plural: 'Files', icon: FileLock2, fields: [
    { key: 'username', label: 'Description' },
    { key: 'notes', label: 'Notes', multiline: true },
  ] },
]
export const kindOf = (k: string) => KINDS.find((x) => x.id === k) ?? KINDS[0]
