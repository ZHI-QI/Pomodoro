import { invoke } from '@tauri-apps/api/core';
import type { Unit } from './panel';

export type { Unit };

export interface SessionDto {
  id: number;
  note: string;
  noteLen: number;
  plannedSec: number;
  actualSec: number | null;
  startedAt: string;
  endedAt: string | null;
  status: 'running' | 'completed' | 'aborted';
}

export interface SettingsDto {
  theme: 'system' | 'dark' | 'light';
  sound: boolean;
  notification: boolean;
  autostart: boolean;
}

export interface StatsDto {
  todaySec: number;
  weekSec: number;
  completedCount: number;
  byDay: { day: string; focusSec: number }[];
}

export interface TickDto {
  sessionId: number;
  remainingSec: number;
  plannedSec: number;
  noteShort: string;
}

export interface DoneDto {
  sessionId: number;
  note: string;
  plannedSec: number;
  noteLen: number;
  sound: boolean;
}

export interface AbortDto {
  sessionId: number;
}

export interface StoreOutcomeDto {
  outcome: 'opened' | 'recovered' | 'reset';
}

export const startSession = (note: string, unit: Unit, amount: number) =>
  invoke<SessionDto>('start_session', { note, unit, amount });
export const abortSession = (id: number) => invoke<SessionDto>('abort_session', { id });
export const getActiveSession = () => invoke<SessionDto | null>('get_active_session');
export const listToday = () => invoke<SessionDto[]>('list_today');
export const listRecent = (limit = 12) => invoke<SessionDto[]>('list_recent', { limit });
export const getStats = () => invoke<StatsDto>('get_stats');
export const getSettings = () => invoke<SettingsDto>('get_settings');
export const saveSettings = (settings: SettingsDto) =>
  invoke<SettingsDto>('set_settings', { settings });
export const getStoreOutcome = () => invoke<StoreOutcomeDto>('get_store_outcome');
