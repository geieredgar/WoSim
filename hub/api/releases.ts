import { invoke } from '@tauri-apps/api/tauri'

export async function releaseNotes() {
  return await invoke('release_notes', {})
}
