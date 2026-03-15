import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
	AdultDisplay,
	AwardDisplay,
	BadgePivotEntry,
	BadgePivotScout,
	BadgeRequirementsResponseDisplay,
	BadgeSummary,
	CacheAges,
	Commissioner,
	EventDisplay,
	EventGuestDisplay,
	Key3Leaders,
	Leader,
	LeadershipDisplay,
	MeetingLocation,
	MeritBadgeDisplay,
	MeritBadgeRequirementDisplay,
	OrgProfile,
	ParentDisplay,
	Patrol,
	RankPivotEntry,
	RankPivotScout,
	RankProgressDisplay,
	RankRequirementDisplay,
	UnitContact,
	UnitInfo,
	YouthBadgesResponse,
	YouthDisplay
} from '$lib/bindings';

// Re-export all generated types
export type {
	AdultDisplay,
	AwardDisplay,
	BadgePivotEntry,
	BadgePivotScout,
	BadgeRequirementsResponseDisplay,
	BadgeSummary,
	CacheAges,
	Commissioner,
	EventDisplay,
	EventGuestDisplay,
	Key3Leaders,
	Leader,
	LeadershipDisplay,
	MeetingLocation,
	MeritBadgeDisplay,
	MeritBadgeRequirementDisplay,
	OrgProfile,
	ParentDisplay,
	Patrol,
	RankPivotEntry,
	RankPivotScout,
	RankProgressDisplay,
	RankRequirementDisplay,
	UnitContact,
	UnitInfo,
	YouthBadgesResponse,
	YouthDisplay
} from '$lib/bindings';

// LoginResponse is from our own Tauri command struct (not a core model)
export interface LoginResponse {
	user_id: number;
	organization_guid: string;
	username: string;
	unit_name: string | null;
}

// RefreshProgress event payload from the backend
export interface RefreshProgress {
	step: string;
	current: number;
	total: number;
	error: string | null;
}

// ============================================================================
// API Functions
// ============================================================================

export async function login(username: string, password: string): Promise<LoginResponse> {
	return invoke('login', { username, password });
}

export async function getSavedUsername(): Promise<string | null> {
	return invoke('get_saved_username');
}

export async function logout(): Promise<void> {
	return invoke('logout');
}

export async function getYouth(): Promise<YouthDisplay[]> {
	return invoke('get_youth');
}

export async function getAdults(): Promise<AdultDisplay[]> {
	return invoke('get_adults');
}

export async function getParents(): Promise<ParentDisplay[]> {
	return invoke('get_parents');
}

export async function getEvents(): Promise<EventDisplay[]> {
	return invoke('get_events');
}

export async function getEventGuests(eventId: number): Promise<EventGuestDisplay[]> {
	return invoke('get_event_guests', { eventId });
}

export async function getPatrols(): Promise<Patrol[]> {
	return invoke('get_patrols');
}

export async function getUnitInfo(): Promise<UnitInfo> {
	return invoke('get_unit_info');
}

export async function getKey3(): Promise<Key3Leaders> {
	return invoke('get_key3');
}

export async function getOrgProfile(): Promise<OrgProfile> {
	return invoke('get_org_profile');
}

export async function getCommissioners(): Promise<Commissioner[]> {
	return invoke('get_commissioners');
}

export async function getYouthRanks(userId: number): Promise<RankProgressDisplay[]> {
	return invoke('get_youth_ranks', { userId });
}

export async function getYouthMeritBadges(userId: number): Promise<YouthBadgesResponse> {
	return invoke('get_youth_merit_badges', { userId });
}

export async function getYouthLeadership(userId: number): Promise<LeadershipDisplay[]> {
	return invoke('get_youth_leadership', { userId });
}

export async function getYouthAwards(userId: number): Promise<AwardDisplay[]> {
	return invoke('get_youth_awards', { userId });
}

export async function getRankRequirements(
	userId: number,
	rankId: number
): Promise<RankRequirementDisplay[]> {
	return invoke('get_rank_requirements', { userId, rankId });
}

export async function getBadgeRequirements(
	userId: number,
	badgeId: number
): Promise<BadgeRequirementsResponseDisplay> {
	return invoke('get_badge_requirements', { userId, badgeId });
}

export async function getAllYouthRanks(): Promise<RankPivotEntry[]> {
	return invoke('get_all_youth_ranks');
}

export async function getAllYouthBadges(): Promise<BadgePivotEntry[]> {
	return invoke('get_all_youth_badges');
}

export async function getCacheAges(): Promise<CacheAges> {
	return invoke('get_cache_ages');
}

export async function refreshData(): Promise<string> {
	return invoke('refresh_data');
}

export async function getOfflineMode(): Promise<boolean> {
	return invoke('get_offline_mode');
}

export async function setOfflineMode(offline: boolean): Promise<boolean> {
	return invoke('set_offline_mode', { offline });
}

export async function cacheForOffline(): Promise<string> {
	return invoke('cache_for_offline');
}

export async function quitApp(): Promise<void> {
	return invoke('quit_app');
}

export async function onRefreshProgress(
	callback: (progress: RefreshProgress) => void
): Promise<UnlistenFn> {
	return listen<RefreshProgress>('refresh-progress', (event) => {
		callback(event.payload);
	});
}
