<script lang="ts">
	import {
		login,
		logout,
		getSavedUsername,
		getYouth,
		getAdults,
		getParents,
		getEvents,
		getEventGuests,
		getPatrols,
		getUnitInfo,
		getKey3,
		getOrgProfile,
		getCommissioners,
		getYouthRanks,
		getYouthMeritBadges,
		getYouthLeadership,
		getYouthAwards,
		getRankRequirements,
		getBadgeRequirements,
		getAllYouthRanks,
		getAllYouthBadges,
		getCacheAges,
		refreshData,
		getOfflineMode,
		setOfflineMode,
		onRefreshProgress,
		type YouthDisplay,
		type AdultDisplay,
		type ParentDisplay,
		type EventDisplay,
		type EventGuestDisplay,
		type UnitInfo,
		type Key3Leaders,
		type Patrol,
		type Commissioner,
		type OrgProfile,
		type RankProgressDisplay,
		type MeritBadgeDisplay,
		type LeadershipDisplay,
		type AwardDisplay,
		type RankRequirementDisplay,
		type RankPivotScout,
		type BadgePivotScout,
		type MeritBadgeRequirementDisplay,
		type BadgeRequirementsResponseDisplay,
		type RankPivotEntry,
		type BadgePivotEntry,
		type CacheAges,
		type RefreshProgress
	} from '$lib/api';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	const tabs = [
		{ id: 'scouts', label: 'Scouts' },
		{ id: 'ranks', label: 'Ranks' },
		{ id: 'badges', label: 'Badges' },
		{ id: 'events', label: 'Events' },
		{ id: 'adults', label: 'Adults' },
		{ id: 'unit', label: 'Unit' }
	];

	// ========================================================================
	// App State
	// ========================================================================

	let loggedIn = $state(false);
	let loading = $state(false);
	let error = $state('');
	let activeTab = $state('scouts');
	let searchQuery = $state('');
	let statusMessage = $state('');
	let statusErrors: string[] = $state([]);
	let offlineMode = $state(false);
	let showOfflineModal = $state(false);
	let confirmQuit = $state(false);
	let refreshProgress: RefreshProgress | null = $state(null);

	// Login
	let username = $state('');
	let password = $state('');
	let loginError = $state('');
	let loggingIn = $state(false);

	// Core data
	let youth: YouthDisplay[] = $state([]);
	let adults: AdultDisplay[] = $state([]);
	let parents: ParentDisplay[] = $state([]);
	let events: EventDisplay[] = $state([]);
	let patrols: Patrol[] = $state([]);
	let unitInfo: UnitInfo | null = $state(null);
	let key3: Key3Leaders | null = $state(null);
	let commissioners: Commissioner[] = $state([]);
	let orgProfile: OrgProfile | null = $state(null);
	let cacheAges: CacheAges | null = $state(null);

	// Scout detail
	let selectedYouth: YouthDisplay | null = $state(null);
	let youthRanks: RankProgressDisplay[] = $state([]);
	let youthBadges: MeritBadgeDisplay[] = $state([]);
	let youthLeadership: LeadershipDisplay[] = $state([]);
	let youthAwards: AwardDisplay[] = $state([]);
	let loadingDetail = $state(false);

	// Scout detail drill-down
	let scoutDetailTab = $state<'details' | 'ranks' | 'badges' | 'leadership' | 'awards'>('details');
	let detailView = $state<'overview' | 'rank-reqs' | 'badge-reqs'>('overview');
	let selectedRankForReqs: RankProgressDisplay | null = $state(null);
	let selectedBadgeForReqs: MeritBadgeDisplay | null = $state(null);
	let rankRequirements: RankRequirementDisplay[] = $state([]);
	let badgeRequirements: MeritBadgeRequirementDisplay[] = $state([]);
	let badgeResponse: BadgeRequirementsResponseDisplay | null = $state(null);
	let loadingRequirements = $state(false);

	// Event detail
	let selectedEvent: EventDisplay | null = $state(null);
	let eventGuests: EventGuestDisplay[] = $state([]);
	let loadingEventDetail = $state(false);
	let eventDetailView = $state<'details' | 'rsvp'>('details');

	// Pivot tabs (pre-aggregated from backend)
	let rankPivotData: RankPivotEntry[] = $state([]);
	let badgePivotData: BadgePivotEntry[] = $state([]);
	let pivotDataLoaded = $state(false);
	let loadingPivotData = $state(false);
	let selectedPivotRank: string | null = $state(null);
	let selectedPivotBadge: string | null = $state(null);
	let pivotRankReqsScout: RankPivotScout | null = $state(null);
	let pivotRankReqs: RankRequirementDisplay[] = $state([]);
	let loadingPivotReqs = $state(false);
	let pivotBadgeReqsScout: BadgePivotScout | null = $state(null);
	let pivotBadgeReqs: MeritBadgeRequirementDisplay[] = $state([]);
	let pivotBadgeResponse: BadgeRequirementsResponseDisplay | null = $state(null);
	let loadingPivotBadgeReqs = $state(false);

	// Mobile navigation
	let mobileDetailOpen = $state(false);

	// Pull-to-refresh
	let pullStartY = $state(0);
	let pulling = $state(false);
	let pullDistance = $state(0);
	const PULL_THRESHOLD = 80;

	// Sorting
	let scoutSortColumn = $state('last_name');
	let scoutSortAsc = $state(true);
	let eventSortColumn = $state('start_date');
	let eventSortAsc = $state(false);
	let adultSortColumn = $state('last_name');
	let adultSortAsc = $state(true);
	let rankPivotSortByCount = $state(false);
	let rankPivotSortAsc = $state(false);
	let badgePivotSortByCount = $state(false);
	let badgePivotSortAsc = $state(true);

	// ========================================================================
	// Lifecycle
	// ========================================================================

	$effect(() => {
		getCurrentWindow().onCloseRequested((e) => {
			e.preventDefault();
			confirmQuit = true;
		});
		getSavedUsername()
			.then((saved) => {
				if (saved) username = saved;
			})
			.catch(() => {});
		getOfflineMode()
			.then((offline) => {
				offlineMode = offline;
				if (offline) statusMessage = 'Offline mode - using cached data';
			})
			.catch(() => {});
	});

	// Lazy-load pivot data when switching to ranks/badges tab
	$effect(() => {
		if (
			(activeTab === 'ranks' || activeTab === 'badges') &&
			!pivotDataLoaded &&
			!loadingPivotData &&
			loggedIn
		) {
			loadPivotData();
		}
	});

	// Keyboard shortcuts (desktop)
	function handleKeydown(e: KeyboardEvent) {
		if (!loggedIn) return;

		const mod = e.ctrlKey || e.metaKey;
		const target = e.target as HTMLElement;
		const inInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA';

		// Block shortcuts while modal is open
		if (showOfflineModal || confirmQuit) {
			if (e.key === 'Escape') { showOfflineModal = false; confirmQuit = false; }
			return;
		}

		// 1-6: switch tabs
		if (!inInput && e.key >= '1' && e.key <= '6') {
			e.preventDefault();
			const idx = parseInt(e.key) - 1;
			if (idx < tabs.length) {
				activeTab = tabs[idx].id;
				selectedYouth = null;
				selectedEvent = null;
				detailView = 'overview';
			}
			return;
		}

		// s: focus search
		if (!inInput && e.key === 's') {
			e.preventDefault();
			const searchEl = document.querySelector('.search-input') as HTMLInputElement;
			searchEl?.focus();
			searchEl?.select();
			return;
		}

		// Escape: clear search, deselect, or back from drill-down
		if (e.key === 'Escape') {
			if (inInput) {
				(target as HTMLInputElement).blur();
				searchQuery = '';
				return;
			}
			if (mobileDetailOpen) {
				mobileBack();
				return;
			}
			if (detailView !== 'overview') {
				backToOverview();
				return;
			}
			if (selectedYouth) {
				selectedYouth = null;
				return;
			}
			if (selectedEvent) {
				selectedEvent = null;
				return;
			}
		}
	}

	// ========================================================================
	// Auth
	// ========================================================================

	async function handleLogin() {
		loginError = '';
		loggingIn = true;
		try {
			await login(username, password);
			loggedIn = true;
			password = '';
			await loadAllData();
		} catch (e: any) {
			loginError = e?.message || String(e);
		} finally {
			loggingIn = false;
		}
	}

	async function handleLogout() {
		await logout().catch(() => {});
		loggedIn = false;
		youth = [];
		adults = [];
		events = [];
		patrols = [];
		unitInfo = null;
		key3 = null;
		commissioners = [];
		orgProfile = null;
		selectedYouth = null;
		selectedEvent = null;
		pivotDataLoaded = false;
		rankPivotData = [];
		badgePivotData = [];
	}

	// ========================================================================
	// Data Loading
	// ========================================================================

	async function loadAllData() {
		loading = true;
		statusMessage = 'Loading data...';
		try {
			const results = await Promise.allSettled([
				getYouth().then((d) => (youth = d)),
				getAdults().then((d) => (adults = d)),
				getParents().then((d) => (parents = d)),
				getEvents().then((d) => (events = d)),
				getPatrols().then((d) => (patrols = d)),
				getUnitInfo().then((d) => (unitInfo = d)),
				getKey3().then((d) => (key3 = d)),
				getCommissioners().then((d) => (commissioners = d)),
				getOrgProfile().then((d) => (orgProfile = d)),
				getCacheAges().then((d) => (cacheAges = d))
			]);
			const failures = results.filter((r): r is PromiseRejectedResult => r.status === 'rejected');
			if (failures.length > 0) {
				statusErrors = failures.map((f) => f.reason?.message || String(f.reason));
				statusMessage = `Loaded with ${failures.length} error(s)`;
			} else {
				statusMessage = `Loaded ${youth.length} scouts, ${adults.length} adults, ${events.length} events`;
			}
		} catch (e: any) {
			statusMessage = 'Error loading data';
			error = e?.message || String(e);
		} finally {
			loading = false;
		}
	}

	async function handleRefresh() {
		loading = true;
		statusMessage = 'Refreshing...';
		statusErrors = [];
		refreshProgress = null;

		// Listen for progress events from the backend
		const unlisten = await onRefreshProgress((progress) => {
			refreshProgress = progress;
			statusMessage = `Refreshing (${progress.current}/${progress.total}): ${progress.step}`;
			if (progress.error) statusErrors.push(progress.error);
		});

		try {
			const result = await refreshData();
			pivotDataLoaded = false;
			refreshProgress = null;
			await loadAllData();
			statusMessage = result;
		} catch (e: any) {
			statusMessage = 'Refresh failed: ' + (e?.message || String(e));
		} finally {
			refreshProgress = null;
			loading = false;
			unlisten();
		}
	}

	async function toggleOfflineMode() {
		try {
			if (offlineMode) {
				// Going online - force re-login (token likely expired)
				offlineMode = await setOfflineMode(false);
				await handleLogout();
			} else {
				// Going offline - cache everything first, then set offline
				statusMessage = 'Caching data for offline mode...';
				await handleRefresh();
				offlineMode = await setOfflineMode(true);
				statusMessage = 'Offline mode - using cached data';
			}
		} catch (e: any) {
			statusMessage = 'Offline toggle failed: ' + (e?.message || String(e));
		}
	}

	async function confirmAndQuit() {
		await getCurrentWindow().destroy();
	}

	async function loadPivotData() {
		loadingPivotData = true;
		try {
			await Promise.allSettled([
				getAllYouthRanks().then((d) => (rankPivotData = d)),
				getAllYouthBadges().then((d) => (badgePivotData = d))
			]);
			pivotDataLoaded = true;
		} catch {}
		loadingPivotData = false;
	}

	// ========================================================================
	// Scout Detail
	// ========================================================================

	async function selectYouth(y: YouthDisplay) {
		selectedYouth = y;
		mobileDetailOpen = true;
		detailView = 'overview';
		scoutDetailTab = 'details';
		loadingDetail = true;
		youthRanks = [];
		youthBadges = [];
		youthLeadership = [];
		youthAwards = [];
		const uid = y.user_id;
		if (uid) {
			try {
				await Promise.allSettled([
					getYouthRanks(uid).then((d) => (youthRanks = d)),
					getYouthMeritBadges(uid).then((d) => (youthBadges = d)),
					getYouthLeadership(uid).then((d) => (youthLeadership = d)),
					getYouthAwards(uid).then((d) => (youthAwards = d))
				]);
			} catch (_) {}
		}
		loadingDetail = false;
	}

	async function viewRankRequirements(rank: RankProgressDisplay) {
		if (!selectedYouth?.user_id) return;
		selectedRankForReqs = rank;
		detailView = 'rank-reqs';
		loadingRequirements = true;
		rankRequirements = [];
		try {
			rankRequirements = await getRankRequirements(selectedYouth.user_id, rank.rank_id);
		} catch {
			rankRequirements = [];
		}
		loadingRequirements = false;
	}

	async function viewBadgeRequirements(badge: MeritBadgeDisplay) {
		if (!selectedYouth?.user_id) return;
		selectedBadgeForReqs = badge;
		detailView = 'badge-reqs';
		loadingRequirements = true;
		badgeRequirements = [];
		badgeResponse = null;
		try {
			badgeResponse = await getBadgeRequirements(selectedYouth.user_id, badge.id);
			badgeRequirements = badgeResponse.requirements;
		} catch {
			badgeRequirements = [];
			badgeResponse = null;
		}
		loadingRequirements = false;
	}

	function backToOverview() {
		detailView = 'overview';
		selectedRankForReqs = null;
		selectedBadgeForReqs = null;
	}

	// ========================================================================
	// Event Detail
	// ========================================================================

	async function selectEvent(e: EventDisplay) {
		selectedEvent = e;
		mobileDetailOpen = true;
		eventDetailView = 'details';
		loadingEventDetail = true;
		eventGuests = [];
		try {
			eventGuests = await getEventGuests(e.id);
		} catch {}
		loadingEventDetail = false;
	}

	async function viewPivotRankRequirements(s: RankPivotScout) {
		if (!s.user_id || !s.rank_id) return;
		pivotRankReqsScout = s;
		loadingPivotReqs = true;
		pivotRankReqs = [];
		try {
			pivotRankReqs = await getRankRequirements(s.user_id, s.rank_id);
		} catch {
			pivotRankReqs = [];
		}
		loadingPivotReqs = false;
	}

	function backFromPivotReqs() {
		pivotRankReqsScout = null;
		pivotRankReqs = [];
	}

	async function viewPivotBadgeRequirements(s: BadgePivotScout) {
		if (!s.user_id || !s.badge_id) return;
		pivotBadgeReqsScout = s;
		loadingPivotBadgeReqs = true;
		pivotBadgeReqs = [];
		pivotBadgeResponse = null;
		try {
			pivotBadgeResponse = await getBadgeRequirements(s.user_id, s.badge_id);
			pivotBadgeReqs = pivotBadgeResponse.requirements;
		} catch {
			pivotBadgeReqs = [];
			pivotBadgeResponse = null;
		}
		loadingPivotBadgeReqs = false;
	}

	function backFromPivotBadgeReqs() {
		pivotBadgeReqsScout = null;
		pivotBadgeReqs = [];
		pivotBadgeResponse = null;
	}

	// ========================================================================
	// Utility
	// ========================================================================

	function matchesSearch(text: string | null | undefined): boolean {
		if (!searchQuery) return true;
		if (!text) return false;
		return text.toLowerCase().includes(searchQuery.toLowerCase());
	}

	// ========================================================================
	// Derived Data
	// ========================================================================

	let filteredYouth = $derived(
		youth
			.filter(
				(y) =>
					matchesSearch(y.display_name) ||
					matchesSearch(y.rank) ||
					matchesSearch(y.patrol)
			)
			.sort((a, b) => {
				let cmp: number;
				if (scoutSortColumn === 'last_name') {
					cmp = a.last_name.localeCompare(b.last_name, undefined, { sensitivity: 'base' });
				} else if (scoutSortColumn === 'first_name') {
					cmp = a.first_name.localeCompare(b.first_name, undefined, { sensitivity: 'base' });
				} else if (scoutSortColumn === 'rank') {
					cmp = a.rank_order - b.rank_order;
				} else if (scoutSortColumn === 'age') {
					cmp = a.age.localeCompare(b.age, undefined, { numeric: true });
				} else if (scoutSortColumn === 'grade') {
					cmp = a.grade.localeCompare(b.grade, undefined, { numeric: true });
				} else {
					cmp = a.patrol.localeCompare(b.patrol, undefined, { sensitivity: 'base' });
				}
				return scoutSortAsc ? cmp : -cmp;
			})
	);

	let filteredEvents = $derived(
		events
			.filter(
				(e) =>
					matchesSearch(e.name) ||
					matchesSearch(e.location) ||
					matchesSearch(e.event_type)
			)
			.sort((a, b) => {
				let va: string, vb: string;
				if (eventSortColumn === 'name') {
					va = a.name;
					vb = b.name;
				} else if (eventSortColumn === 'start_date') {
					va = a.start_date || '';
					vb = b.start_date || '';
				} else if (eventSortColumn === 'location') {
					va = a.location || '';
					vb = b.location || '';
				} else {
					va = a.derived_type;
					vb = b.derived_type;
				}
				const cmp = va.localeCompare(vb);
				return eventSortAsc ? cmp : -cmp;
			})
	);

	let filteredAdults = $derived(
		adults
			.filter(
				(a) =>
					matchesSearch(a.display_name) ||
					matchesSearch(a.role) ||
					matchesSearch(a.email)
			)
			.sort((a, b) => {
				let va: string, vb: string;
				if (adultSortColumn === 'last_name') {
					va = a.last_name;
					vb = b.last_name;
				} else if (adultSortColumn === 'position') {
					va = a.role;
					vb = b.role;
				} else {
					va = a.email || '';
					vb = b.email || '';
				}
				const cmp = va.localeCompare(vb, undefined, { sensitivity: 'base' });
				return adultSortAsc ? cmp : -cmp;
			})
	);

	// Badge summary for scout detail
	let badgeSummary = $derived.by(() => {
		const completed = youthBadges.filter((b) => b.is_completed).length;
		const inProgress = youthBadges.filter((b) => !b.is_completed).length;
		const eagle = youthBadges.filter((b) => b.is_eagle_required && b.is_completed).length;
		return { completed, inProgress, eagle };
	});

	// Pivot data (pre-aggregated from backend)
	let filteredRankPivot = $derived(
		rankPivotData
			.filter((r) => matchesSearch(r.rank_name))
			.sort((a, b) => {
				let cmp: number;
				if (rankPivotSortByCount) {
					cmp = a.count - b.count;
				} else {
					cmp = a.rank_order - b.rank_order;
				}
				return rankPivotSortAsc ? cmp : -cmp;
			})
	);


	function pivotScoutOrder(a: { is_awarded: boolean; is_completed: boolean; sort_date: string }, b: { is_awarded: boolean; is_completed: boolean; sort_date: string }): number {
		const statusOrder = (s: { is_awarded: boolean; is_completed: boolean }) =>
			s.is_awarded ? 2 : s.is_completed ? 1 : 0;
		const diff = statusOrder(a) - statusOrder(b);
		if (diff !== 0) return diff;
		return b.sort_date.localeCompare(a.sort_date);
	}

	let selectedRankScouts = $derived.by(() => {
		if (!selectedPivotRank) return [];
		const entry = rankPivotData.find((r) => r.rank_name === selectedPivotRank);
		return [...(entry?.scouts ?? [])].sort(pivotScoutOrder);
	});

	let filteredBadgePivot = $derived(
		badgePivotData
			.filter((b) => matchesSearch(b.badge_name))
			.sort((a, b) => {
				let cmp: number;
				if (badgePivotSortByCount) {
					cmp = a.count - b.count;
				} else {
					cmp = a.badge_name.localeCompare(b.badge_name);
				}
				return badgePivotSortAsc ? cmp : -cmp;
			})
	);

	let selectedBadgeScouts = $derived.by(() => {
		if (!selectedPivotBadge) return [];
		const entry = badgePivotData.find((b) => b.badge_name === selectedPivotBadge);
		return [...(entry?.scouts ?? [])].sort(pivotScoutOrder);
	});

	// Event guests derived (status pre-computed by backend)
	let adultGuests = $derived(eventGuests.filter((g) => !g.is_youth));
	let youthGuests = $derived(eventGuests.filter((g) => g.is_youth));
	let rsvpYesAdults = $derived(adultGuests.filter((g) => g.status === 'Going'));
	let rsvpNoAdults = $derived(adultGuests.filter((g) => g.status === 'Not Going'));
	let rsvpYesYouth = $derived(youthGuests.filter((g) => g.status === 'Going'));
	let rsvpNoYouth = $derived(youthGuests.filter((g) => g.status === 'Not Going'));

	// Parents for selected youth
	let selectedYouthParents: ParentDisplay[] = $derived.by(() => {
		const uid = selectedYouth?.user_id;
		if (!uid) return [];
		return parents.filter((p) => p.youth_user_id === uid);
	});

	// Unit tab computed data
	let scoutRenewals = $derived(youth.filter((y) => y.membership_style === 'expired' || y.membership_style === 'expiring').sort((a, b) => (a.membership_sort_date || '').localeCompare(b.membership_sort_date || '')));
	let adultRenewals = $derived(adults.filter((a) => a.membership_style === 'expired' || a.membership_style === 'expiring').sort((a, b) => (a.membership_sort_date || '').localeCompare(b.membership_sort_date || '')));
	let yptIssues = $derived(adults.filter((a) => a.ypt_style === 'expired' || a.ypt_style === 'expiring').sort((a, b) => (a.ypt_sort_date || '').localeCompare(b.ypt_sort_date || '')));
	let notTrained = $derived(adults.filter((a) => a.position_trained === 'Not Trained'));
	let youthPositions = $derived(
		youth
			.filter((y) => y.position && y.position !== 'Scouts BSA' && y.position !== 'Scout')
			.map((y) => ({
				position: y.position!,
				display: (y.position === 'Patrol Leader' || y.position === 'Assistant Patrol Leader') && y.patrol
					? y.position + ' (' + y.patrol + ')'
					: y.position!,
				name: y.display_name
			}))
			.sort((a, b) => {
				const order = ['Senior Patrol Leader', 'Assistant Senior Patrol Leader', 'Troop Guide', 'Patrol Leader', 'Assistant Patrol Leader', 'Quartermaster', 'Scribe', 'Historian', 'Librarian', 'Chaplain Aide', 'Outdoor Ethics Guide', 'Den Chief', 'Instructor', 'Junior Assistant Scoutmaster', 'Bugler'];
				const ai = order.indexOf(a.position); const bi = order.indexOf(b.position);
				return (ai === -1 ? 999 : ai) - (bi === -1 ? 999 : bi) || a.display.localeCompare(b.display) || a.name.localeCompare(b.name);
			})
	);

	// ========================================================================
	// Sort Helpers
	// ========================================================================

	function toggleScoutSort(col: string) {
		if (scoutSortColumn === col) scoutSortAsc = !scoutSortAsc;
		else {
			scoutSortColumn = col;
			scoutSortAsc = true;
		}
	}

	function toggleEventSort(col: string) {
		if (eventSortColumn === col) eventSortAsc = !eventSortAsc;
		else {
			eventSortColumn = col;
			eventSortAsc = true;
		}
	}

	function toggleAdultSort(col: string) {
		if (adultSortColumn === col) adultSortAsc = !adultSortAsc;
		else {
			adultSortColumn = col;
			adultSortAsc = true;
		}
	}

	function sortIndicator(col: string, activeCol: string, asc: boolean): string {
		if (col !== activeCol) return '';
		return asc ? ' \u25B2' : ' \u25BC';
	}

	function toggleRankPivotSort(byCount: boolean) {
		if (rankPivotSortByCount === byCount) {
			rankPivotSortAsc = !rankPivotSortAsc;
		} else {
			rankPivotSortByCount = byCount;
			rankPivotSortAsc = byCount ? false : true;
		}
	}

	function toggleBadgePivotSort(byCount: boolean) {
		if (badgePivotSortByCount === byCount) {
			badgePivotSortAsc = !badgePivotSortAsc;
		} else {
			badgePivotSortByCount = byCount;
			badgePivotSortAsc = byCount ? false : true;
		}
	}

	let calendarUrl = $derived.by(() => {
		const firstWithUnit = events.find((e) => e.unit_id != null);
		if (!firstWithUnit?.unit_id) return null;
		return `https://api.scouting.org/advancements/events/calendar/${firstWithUnit.unit_id}`;
	});


	// ========================================================================
	// Mobile Navigation
	// ========================================================================

	function mobileBack() {
		if (detailView !== 'overview') {
			backToOverview();
		} else {
			mobileDetailOpen = false;
			selectedYouth = null;
			selectedEvent = null;
			selectedPivotRank = null;
			selectedPivotBadge = null;
		}
	}

	// Pull-to-refresh touch handlers
	function handleTouchStart(e: TouchEvent) {
		const target = e.target as HTMLElement;
		const scrollParent = target.closest('.list-panel, .single-panel');
		if (scrollParent && scrollParent.scrollTop > 0) return;
		pullStartY = e.touches[0].clientY;
		pulling = true;
	}

	function handleTouchMove(e: TouchEvent) {
		if (!pulling) return;
		const dy = e.touches[0].clientY - pullStartY;
		if (dy > 0) {
			pullDistance = Math.min(dy * 0.5, PULL_THRESHOLD * 1.5);
		} else {
			pullDistance = 0;
		}
	}

	function handleTouchEnd() {
		if (pulling && pullDistance >= PULL_THRESHOLD && !loading) {
			handleRefresh();
		}
		pulling = false;
		pullDistance = 0;
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if !loggedIn}
	<!-- Login Screen -->
	<div class="login-container">
		<div class="login-box">
			<h1 class="login-title">Trailcache</h1>
			{#if offlineMode}
				<p class="login-subtitle">Offline Mode</p>
				<p class="offline-login-note">Password is used to decrypt cached data</p>
			{:else}
				<p class="login-subtitle">Scout Troop Management</p>
			{/if}

			<form
				onsubmit={(e) => {
					e.preventDefault();
					handleLogin();
				}}
			>
				<label class="field-label">
					Username
					<input
						type="text"
						bind:value={username}
						placeholder="my.scouting.org username"
						disabled={loggingIn}
						autocomplete="username"
					/>
				</label>

				<label class="field-label">
					Password
					<input
						type="password"
						bind:value={password}
						placeholder="Password"
						disabled={loggingIn}
						autocomplete="current-password"
					/>
				</label>

				{#if loginError}
					<div class="error-message">{loginError}</div>
				{/if}

				<button
					type="submit"
					class="login-button"
					disabled={loggingIn || !username || !password}
				>
					{loggingIn ? 'Signing in...' : offlineMode ? 'Unlock Cache' : 'Sign In'}
				</button>
			</form>
		</div>
	</div>
{:else}
	<!-- Main App -->
	<div
		class="app-layout"
		ontouchstart={handleTouchStart}
		ontouchmove={handleTouchMove}
		ontouchend={handleTouchEnd}
	>
		<!-- Pull-to-refresh indicator -->
		{#if pullDistance > 0}
			<div class="pull-indicator" style="height: {pullDistance}px">
				<span class="pull-text">
					{pullDistance >= PULL_THRESHOLD ? 'Release to refresh' : 'Pull to refresh'}
				</span>
			</div>
		{/if}

		<!-- Top Bar -->
		<header class="top-bar">
			<div class="top-bar-left">
				{#if mobileDetailOpen}
					<button class="mobile-back-btn" onclick={mobileBack}>
						&larr;
					</button>
				{/if}
				<span class="app-name">Trailcache</span>
				<nav class="tab-nav desktop-only">
					{#each tabs as tab}
						<button
							class="tab-button"
							class:active={activeTab === tab.id}
							onclick={() => {
								activeTab = tab.id;
								mobileDetailOpen = false;
								selectedYouth = null;
								selectedEvent = null;
								detailView = 'overview';
							}}
						>
							{tab.label}
						</button>
					{/each}
				</nav>
			</div>
			<div class="top-bar-right">
				<input
					type="text"
					class="search-input"
					placeholder="Search..."
					bind:value={searchQuery}
				/>
				<button
					class="icon-button"
					onclick={() => { if (!offlineMode) handleRefresh(); }}
					disabled={loading || offlineMode}
					title="Update data"
				>
					{loading ? '...' : '\u21BB'}
				</button>
				<button
					class="icon-button {offlineMode ? 'offline-active' : ''}"
					onclick={() => { showOfflineModal = true; }}
					title={offlineMode ? 'Go online' : 'Go offline'}
				>
					{offlineMode ? '\u2300' : '\u2601'}
				</button>
				<button class="icon-button" onclick={handleLogout} title="Sign out">
					&#x23FB;
				</button>
			</div>
		</header>

		<!-- Content -->
		<main class="content">
			<!-- ============================================================ -->
			<!-- SCOUTS TAB                                                   -->
			<!-- ============================================================ -->
			{#if activeTab === 'scouts'}
				<div class="split-view">
					<div class="list-panel" class:mobile-hidden={mobileDetailOpen}>
						<table class="data-table">
							<thead>
								<tr>
									<th
										onclick={() => toggleScoutSort('last_name')}
										class="sortable"
									>
										Name{sortIndicator(
											'last_name',
											scoutSortColumn,
											scoutSortAsc
										)}
									</th>
									<th onclick={() => toggleScoutSort('patrol')} class="sortable">
										Patrol{sortIndicator(
											'patrol',
											scoutSortColumn,
											scoutSortAsc
										)}
									</th>
									<th onclick={() => toggleScoutSort('rank')} class="sortable">
										Rank{sortIndicator('rank', scoutSortColumn, scoutSortAsc)}
									</th>
									<th onclick={() => toggleScoutSort('grade')} class="sortable hide-mobile">
										Grade{sortIndicator('grade', scoutSortColumn, scoutSortAsc)}
									</th>
									<th onclick={() => toggleScoutSort('age')} class="sortable hide-mobile">
										Age{sortIndicator('age', scoutSortColumn, scoutSortAsc)}
									</th>
								</tr>
							</thead>
							<tbody>
								{#each filteredYouth as y}
									<tr
										class:selected={selectedYouth?.user_id === y.user_id}
										onclick={() => selectYouth(y)}
									>
										<td>{y.display_name}</td>
										<td>{y.patrol}</td>
										<td>{y.rank}</td>
										<td class="hide-mobile">{y.grade}</td>
										<td class="hide-mobile">{y.age}</td>
									</tr>
								{/each}
								{#if filteredYouth.length === 0}
									<tr
										><td colspan="5" class="empty-message"
											>No scouts found</td
										></tr
									>
								{/if}
							</tbody>
						</table>
					</div>

					<div class="detail-panel" class:mobile-hidden={!mobileDetailOpen}>
						{#if selectedYouth}
							{#if detailView === 'overview'}
								<!-- Scout Overview -->
								<h2 class="detail-title">
									{selectedYouth.short_name}
								</h2>
								<div class="detail-meta">
									<span class="badge">{selectedYouth.rank}</span>
									{#if selectedYouth.patrol !== '-'}<span
											class="badge badge-muted"
											>{selectedYouth.patrol}</span
										>{/if}
									{#if selectedYouth.position}<span class="badge badge-muted"
											>{selectedYouth.position}</span
										>{/if}
								</div>

								<div class="detail-tabs">
									<button class="detail-tab" class:active={scoutDetailTab === 'details'} onclick={() => (scoutDetailTab = 'details')}>Details</button>
									<button class="detail-tab" class:active={scoutDetailTab === 'ranks'} onclick={() => (scoutDetailTab = 'ranks')}>Ranks</button>
									<button class="detail-tab" class:active={scoutDetailTab === 'badges'} onclick={() => (scoutDetailTab = 'badges')}>Merit Badges</button>
									<button class="detail-tab" class:active={scoutDetailTab === 'leadership'} onclick={() => (scoutDetailTab = 'leadership')}>Leadership</button>
									<button class="detail-tab" class:active={scoutDetailTab === 'awards'} onclick={() => (scoutDetailTab = 'awards')}>Awards</button>
								</div>
								{#if loadingDetail}
									<p class="loading-text">Loading details...</p>
								{:else}
									{#if scoutDetailTab === 'details'}
										<section class="detail-section">
											<h3>Unit Info</h3>
											<div class="info-grid">
												<div class="info-item">
													<span class="info-label">Patrol</span>
													<span class="info-value">{selectedYouth.patrol}</span>
												</div>
												<div class="info-item">
													<span class="info-label">Rank</span>
													<span class="info-value">{selectedYouth.rank}</span>
												</div>
												{#if selectedYouth.position}
													<div class="info-item">
														<span class="info-label">Position</span>
														<span class="info-value">{selectedYouth.position}</span>
													</div>
												{/if}
												{#if selectedYouth.membership_status}
													<div class="info-item">
														<span class="info-label">Membership</span>
														<span class="info-value" class:membership-expired={selectedYouth.membership_style === 'expired'} class:membership-active={selectedYouth.membership_style === 'active'}>{selectedYouth.membership_status}</span>
													</div>
												{/if}
												{#if selectedYouth.member_id}
													<div class="info-item">
														<span class="info-label">BSA ID</span>
														<span class="info-value">{selectedYouth.member_id}</span>
													</div>
												{/if}
											</div>
										</section>
										<section class="detail-section">
											<h3>Basic Info</h3>
											<div class="info-grid">
												<div class="info-item">
													<span class="info-label">Age</span>
													<span class="info-value">{selectedYouth.age}{#if selectedYouth.date_of_birth} (born {selectedYouth.date_of_birth}){/if}</span>
												</div>
												{#if selectedYouth.gender}
													<div class="info-item">
														<span class="info-label">Gender</span>
														<span class="info-value">{selectedYouth.gender}</span>
													</div>
												{/if}
												<div class="info-item">
													<span class="info-label">Grade</span>
													<span class="info-value">{selectedYouth.grade}</span>
												</div>
											</div>
										</section>
										<section class="detail-section">
											<h3>Contact</h3>
											<div class="info-grid">
												{#if selectedYouth.phone}
													<div class="info-item">
														<span class="info-label">Phone</span>
														<span class="info-value">{selectedYouth.phone}</span>
													</div>
												{/if}
												{#if selectedYouth.email}
													<div class="info-item">
														<span class="info-label">Email</span>
														<span class="info-value">{selectedYouth.email}</span>
													</div>
												{/if}
												{#if selectedYouth.address_line1}
													<div class="info-item">
														<span class="info-label">Address</span>
														<span class="info-value">{selectedYouth.address_line1}{#if selectedYouth.address_line2}<br/>{selectedYouth.address_line2}{/if}</span>
													</div>
												{/if}
											</div>
										</section>
										{#if selectedYouthParents.length > 0}
											<section class="detail-section">
												<h3>Parents/Guardians</h3>
												{#each selectedYouthParents as parent}
													<div class="parent-card">
														<div class="parent-name">{parent.name}</div>
														<div class="info-grid">
															{#if parent.phone}
																<div class="info-item">
																	<span class="info-label">Phone</span>
																	<span class="info-value">{parent.phone}</span>
																</div>
															{/if}
															{#if parent.email}
																<div class="info-item">
																	<span class="info-label">Email</span>
																	<span class="info-value">{parent.email}</span>
																</div>
															{/if}
															{#if parent.address_line1}
																<div class="info-item">
																	<span class="info-label">Address</span>
																	<span class="info-value">{parent.address_line1}{#if parent.address_line2}<br/>{parent.address_line2}{/if}</span>
																</div>
															{/if}
														</div>
													</div>
												{/each}
											</section>
										{/if}
									{/if}

									{#if scoutDetailTab === 'ranks'}
									<!-- Ranks -->
									{#if youthRanks.length > 0}
										<section class="detail-section">
											<h3>Ranks</h3>
											<table class="detail-table clickable-rows">
												<thead>
													<tr><th>Rank</th><th>Completed</th><th>Progress</th></tr>
												</thead>
												<tbody>
													{#each [...youthRanks].sort((a, b) => b.sort_order - a.sort_order) as r}
														<tr
															class="clickable-row"
															ondblclick={() => viewRankRequirements(r)}
															title="View requirements"
														>
															<td>{r.rank_name}</td>
															<td>{r.is_awarded ? (r.formatted_date_awarded || r.formatted_date_completed) : r.formatted_date_completed}</td>
															<td>
											{#if r.is_awarded}
												<span class="awarded-text">Awarded</span>
											{:else if r.progress_percent != null && !r.is_completed}
																	<div class="progress-bar">
																		<div
																			class="progress-fill"
																			style="width: {r.progress_percent}%"
																		></div>
																	</div>
																	<span class="progress-text"
																		>{r.progress_percent}%</span
																	>
																{:else if r.is_completed}
																	<span class="completed-text"
																		>Completed</span
																	>
																{/if}
															</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</section>
									{/if}
									{/if}

									{#if scoutDetailTab === 'badges'}

									<!-- Merit Badges -->
									{#if youthBadges.length > 0}
										<section class="detail-section">
											<h3>
												Merit Badges ({youthBadges.length})
												<span class="section-meta">
													{badgeSummary.completed} complete, {badgeSummary.inProgress}
													in progress, {badgeSummary.eagle}/13 Eagle
												</span>
											</h3>
											<table class="detail-table clickable-rows">
												<thead>
													<tr><th>Badge</th><th>Status</th><th>Progress</th></tr>
												</thead>
												<tbody>
													{#each [...youthBadges].sort((a, b) => { const statusOrder = (x: typeof a) => x.is_awarded ? 2 : x.is_completed ? 1 : 0; const diff = statusOrder(a) - statusOrder(b); if (diff !== 0) return diff; if (!a.is_completed && !a.is_awarded) return (b.percent_completed ?? 0) - (a.percent_completed ?? 0); return b.sort_date.localeCompare(a.sort_date); }) as b}
														<tr
															class="clickable-row"
															ondblclick={() => viewBadgeRequirements(b)}
															title="View requirements"
														>
															<td>
																{b.name}
																{#if b.is_eagle_required}<span
																		class="eagle-marker"
																		title="Eagle required">E</span
																	>{/if}
															</td>
															<td
																>{b.is_completed
																	? b.formatted_date_completed
																	: b.status}</td
															>
															<td>
											{#if b.is_awarded}
												<span class="awarded-text">Awarded</span>
											{:else if b.progress_percent != null && !b.is_completed}
																	<div class="progress-bar">
																		<div
																			class="progress-fill"
																			style="width: {b.progress_percent}%"
																		></div>
																	</div>
																	<span class="progress-text"
																		>{b.progress_percent}%</span
																	>
																{:else if b.is_completed}
																	<span class="completed-text"
																		>Completed</span
																	>
																{/if}
															</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</section>
									{/if}
									{/if}

									{#if scoutDetailTab === 'leadership'}

									<!-- Leadership -->
									{#if youthLeadership.length > 0}
										<section class="detail-section">
											<h3>Leadership</h3>
											<table class="detail-table">
												<thead>
													<tr
														><th>Position</th><th>Period</th><th
															>Days</th
														></tr
													>
												</thead>
												<tbody>
													{#each [...youthLeadership].sort((a, b) => b.sort_date.localeCompare(a.sort_date)) as l}
														<tr>
															<td
																>{l.name}{#if l.patrol}
																	<span class="text-muted">
																		({l.patrol})</span
																	>{/if}</td
															>
															<td>{l.date_range}</td>
															<td>{l.days_display}</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</section>
									{/if}
									{/if}

									{#if scoutDetailTab === 'awards'}

									<!-- Awards -->
									{#if youthAwards.length > 0}
										<section class="detail-section">
											<h3>Awards ({youthAwards.filter(a => a.is_awarded).length})</h3>
											<table class="detail-table">
												<thead>
													<tr><th>Award</th><th>Date</th><th>Status</th></tr>
												</thead>
												<tbody>
													{#each youthAwards.filter(a => a.is_awarded) as a}
														<tr>
															<td>{a.name}</td>
															<td>{a.date_display}</td>
															<td>{a.status}</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</section>
									{/if}

									{/if}
								{/if}
							{:else if detailView === 'rank-reqs' && selectedRankForReqs}
								<!-- Rank Requirements Drill-Down -->
								<button class="back-button" onclick={backToOverview}>
									&larr; Back
								</button>
								<h2 class="detail-title">
									{selectedRankForReqs.rank_name}
								</h2>
								<p class="detail-subtitle">
									{selectedYouth.short_name}
									{#if selectedRankForReqs.requirements_completed != null && selectedRankForReqs.requirements_total != null}
										&mdash; {selectedRankForReqs.requirements_completed}/{selectedRankForReqs.requirements_total}
										requirements
									{/if}
								</p>
								{#if loadingRequirements}
									<p class="loading-text">Loading requirements...</p>
								{:else if rankRequirements.length > 0}
									<div class="requirements-list">
										{#each rankRequirements as req}
											<div
												class="requirement-item"
												class:completed={req.is_completed}
											>
												<span class="req-check"
													>{req.is_completed
														? '\u2713'
														: '\u25CB'}</span
												>
												<span class="req-number">{req.number}</span>
												<span class="req-text">{req.text}</span>
												{#if req.is_completed}
													<span class="req-date"
														>{req.formatted_date_completed}</span
													>
												{/if}
											</div>
										{/each}
									</div>
								{:else}
									<p class="empty-message">No requirements data available</p>
								{/if}
							{:else if detailView === 'badge-reqs' && selectedBadgeForReqs}
								<!-- Badge Requirements Drill-Down -->
								<button class="back-button" onclick={backToOverview}>
									&larr; Back
								</button>
								<h2 class="detail-title">
									{selectedBadgeForReqs.name}
									{#if selectedBadgeForReqs.is_eagle_required}<span
											class="eagle-marker">E</span
										>{/if}
								</h2>
								<p class="detail-subtitle">
									{selectedYouth.short_name}
								</p>
								{#if badgeResponse && badgeResponse.counselor_name}
									<div class="counselor-info">
										<span class="info-label">Counselor:</span>
										{badgeResponse.counselor_name}
										{#if badgeResponse.counselor_phone}
											<span class="text-muted">
												&middot; {badgeResponse.counselor_phone}</span
											>
										{/if}
										{#if badgeResponse.counselor_email}
											<span class="text-muted">
												&middot; {badgeResponse.counselor_email}</span
											>
										{/if}
									</div>
								{/if}
								{#if loadingRequirements}
									<p class="loading-text">Loading requirements...</p>
								{:else if badgeRequirements.length > 0}
									<div class="requirements-list">
										{#each badgeRequirements as req}
											<div
												class="requirement-item"
												class:completed={req.is_completed}
											>
												<span class="req-check"
													>{req.is_completed
														? '\u2713'
														: '\u25CB'}</span
												>
												<span class="req-number">{req.number}</span>
												<span class="req-text">{req.text}</span>
												{#if req.is_completed}
													<span class="req-date"
														>{req.formatted_date_completed}</span
													>
												{/if}
											</div>
										{/each}
									</div>
								{:else}
									<p class="empty-message">No requirements data available</p>
								{/if}
							{/if}
						{:else}
							<p class="empty-message">Select a scout to view details</p>
						{/if}
					</div>
				</div>

				<!-- ============================================================ -->
				<!-- EVENTS TAB                                                   -->
				<!-- ============================================================ -->
			{:else if activeTab === 'events'}
				<div class="split-view">
					<div class="list-panel" class:mobile-hidden={mobileDetailOpen}>
						<table class="data-table">
							<thead>
								<tr>
									<th
										onclick={() => toggleEventSort('name')}
										class="sortable"
									>
										Event{sortIndicator(
											'name',
											eventSortColumn,
											eventSortAsc
										)}
									</th>
									<th
										onclick={() => toggleEventSort('start_date')}
										class="sortable"
									>
										Date{sortIndicator(
											'start_date',
											eventSortColumn,
											eventSortAsc
										)}
									</th>
									<th onclick={() => toggleEventSort('location')} class="sortable">Location{sortIndicator('location', eventSortColumn, eventSortAsc)}</th>
									<th onclick={() => toggleEventSort('derived_type')} class="sortable">Type{sortIndicator('derived_type', eventSortColumn, eventSortAsc)}</th>
								</tr>
							</thead>
							<tbody>
								{#each filteredEvents as e}
									<tr
										class:selected={selectedEvent?.id === e.id}
										onclick={() => selectEvent(e)}
									>
										<td>{e.name}</td>
										<td>{e.formatted_date}</td>
										<td>{e.location || ''}</td>
										<td>{e.derived_type}</td>
									</tr>
								{/each}
								{#if filteredEvents.length === 0}
									<tr
										><td colspan="4" class="empty-message"
											>No events found</td
										></tr
									>
								{/if}
							</tbody>
						</table>
					</div>

					<div class="detail-panel" class:mobile-hidden={!mobileDetailOpen}>
						{#if selectedEvent}
							<h2 class="detail-title">{selectedEvent.name}</h2>
							<div class="view-toggle">
								<button
									class="toggle-button"
									class:active={eventDetailView === 'details'}
									onclick={() => (eventDetailView = 'details')}
								>
									Details
								</button>
								<button
									class="toggle-button"
									class:active={eventDetailView === 'rsvp'}
									onclick={() => (eventDetailView = 'rsvp')}
								>
									RSVP
									{#if rsvpYesAdults.length + rsvpNoAdults.length + rsvpYesYouth.length + rsvpNoYouth.length > 0}
										({rsvpYesAdults.length + rsvpNoAdults.length + rsvpYesYouth.length + rsvpNoYouth.length})
									{/if}
								</button>
							</div>

							{#if eventDetailView === 'details'}
								<div class="event-detail">
									<div class="info-grid">
										{#if selectedEvent.start_date}
											<div class="info-item">
												<span class="info-label">Start</span>
												<span>{selectedEvent.formatted_start}</span>
											</div>
										{/if}
										{#if selectedEvent.end_date}
											<div class="info-item">
												<span class="info-label">End</span>
												<span>{selectedEvent.formatted_end}</span>
											</div>
										{/if}
										{#if selectedEvent.location}
											<div class="info-item">
												<span class="info-label">Location</span>
												<span>{selectedEvent.location}</span>
											</div>
										{/if}
										<div class="info-item">
											<span class="info-label">Type</span>
											<span>{selectedEvent.derived_type}</span>
										</div>
									</div>
									{#if selectedEvent.description_text}
										<div class="event-description">
											<h3>Description</h3>
											<p class="description-text">
												{selectedEvent.description_text}
											</p>
										</div>
									{/if}
								</div>
							{:else}
								<!-- RSVP View -->
								{#if loadingEventDetail}
									<p class="loading-text">Loading RSVP data...</p>
								{:else if eventGuests.length === 0}
									<p class="empty-message">No RSVP data available</p>
								{:else}
									<div class="rsvp-view">
										{#if rsvpYesAdults.length > 0 || rsvpNoAdults.length > 0}
											<section class="rsvp-section">
												<h3>
													Adults
													<span class="rsvp-counts">
														<span class="rsvp-yes">{rsvpYesAdults.length} Yes</span>
														<span class="rsvp-no">{rsvpNoAdults.length} No</span>
													</span>
												</h3>
												{#each rsvpYesAdults as g}
													<div class="rsvp-item rsvp-going">
														<span class="rsvp-indicator yes">Y</span>
														{g.display_name}
													</div>
												{/each}
												{#each rsvpNoAdults as g}
													<div class="rsvp-item rsvp-not-going">
														<span class="rsvp-indicator no">N</span>
														{g.display_name}
													</div>
												{/each}
											</section>
										{/if}
										{#if rsvpYesYouth.length > 0 || rsvpNoYouth.length > 0}
											<section class="rsvp-section">
												<h3>
													Scouts
													<span class="rsvp-counts">
														<span class="rsvp-yes">{rsvpYesYouth.length} Yes</span>
														<span class="rsvp-no">{rsvpNoYouth.length} No</span>
													</span>
												</h3>
												{#each rsvpYesYouth as g}
													<div class="rsvp-item rsvp-going">
														<span class="rsvp-indicator yes">Y</span>
														{g.display_name}
													</div>
												{/each}
												{#each rsvpNoYouth as g}
													<div class="rsvp-item rsvp-not-going">
														<span class="rsvp-indicator no">N</span>
														{g.display_name}
													</div>
												{/each}
											</section>
										{/if}
									</div>
								{/if}
							{/if}
						{:else}
							<p class="empty-message">Select an event to view details</p>
						{/if}
					</div>
				</div>

				<!-- ============================================================ -->
				<!-- RANKS PIVOT TAB                                              -->
				<!-- ============================================================ -->
			{:else if activeTab === 'ranks'}
				<div class="split-view">
					<div class="list-panel" class:mobile-hidden={mobileDetailOpen}>
						{#if loadingPivotData}
							<p class="loading-text panel-message">
								Loading rank data for all scouts...
							</p>
						{:else if filteredRankPivot.length === 0}
							<p class="empty-message panel-message">No rank data available</p>
						{:else}
							<table class="data-table">
								<thead>
									<tr>
										<th onclick={() => toggleRankPivotSort(false)} class="sortable">Rank{sortIndicator(rankPivotSortByCount ? '' : 'name', rankPivotSortByCount ? '' : 'name', rankPivotSortAsc)}</th>
										<th onclick={() => toggleRankPivotSort(true)} class="sortable">Scouts{sortIndicator(rankPivotSortByCount ? 'count' : '', rankPivotSortByCount ? 'count' : '', rankPivotSortAsc)}</th>
									</tr>
								</thead>
								<tbody>
									{#each filteredRankPivot as r}
										<tr
											class:selected={selectedPivotRank === r.rank_name}
											onclick={() => { selectedPivotRank = r.rank_name; mobileDetailOpen = true; pivotRankReqsScout = null; pivotRankReqs = []; }}
										>
											<td>{r.rank_name}</td>
											<td>{r.count}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}
					</div>

					<div class="detail-panel" class:mobile-hidden={!mobileDetailOpen}>
						{#if pivotRankReqsScout}
							<button class="back-button" onclick={backFromPivotReqs}>
								&larr; Back
							</button>
							<h2 class="detail-title">{selectedPivotRank}</h2>
							<p class="detail-subtitle">{pivotRankReqsScout.display_name}</p>
							{#if loadingPivotReqs}
								<p class="loading-text">Loading requirements...</p>
							{:else if pivotRankReqs.length > 0}
								<div class="requirements-list">
									{#each pivotRankReqs as req}
										<div class="requirement-item" class:completed={req.is_completed}>
											<span class="req-check">{req.is_completed ? '\u2713' : '\u25CB'}</span>
											<span class="req-number">{req.number}</span>
											<span class="req-text">{req.text}</span>
											{#if req.is_completed}
												<span class="req-date">{req.formatted_date_completed}</span>
											{/if}
										</div>
									{/each}
								</div>
							{:else}
								<p class="empty-message">No requirements data available</p>
							{/if}
						{:else if selectedPivotRank}
							<h2 class="detail-title">
								{selectedPivotRank}
								<span class="count-badge">{selectedRankScouts.length} scouts</span>
							</h2>
							{#if selectedRankScouts.length > 0}
								<table class="detail-table clickable-rows">
									<thead>
										<tr><th>Scout</th><th>Status</th></tr>
									</thead>
									<tbody>
										{#each selectedRankScouts as s}
											<tr class="clickable-row" ondblclick={() => viewPivotRankRequirements(s)} title="Double-click to view requirements">
												<td>{s.display_name}</td>
												<td>
													{#if s.is_awarded}
														<span class="awarded-text">Awarded {s.formatted_date_awarded}</span>
													{:else if s.is_completed}
														<span class="completed-text">Completed {s.formatted_date_completed}</span>
													{:else if s.percent_completed != null}
														<div class="progress-bar"><div class="progress-fill" style="width: {s.percent_completed}%"></div></div>
														<span class="progress-text">{Math.round(s.percent_completed)}%</span>
													{/if}
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
							{/if}
						{:else}
							<p class="empty-message">Select a rank to view scouts</p>
						{/if}
					</div>
				</div>

				<!-- ============================================================ -->
				<!-- BADGES PIVOT TAB                                             -->
				<!-- ============================================================ -->
			{:else if activeTab === 'badges'}
				<div class="split-view">
					<div class="list-panel" class:mobile-hidden={mobileDetailOpen}>
						{#if loadingPivotData}
							<p class="loading-text panel-message">
								Loading badge data for all scouts...
							</p>
						{:else if filteredBadgePivot.length === 0}
							<p class="empty-message panel-message">
								No badge data available
							</p>
						{:else}
							<table class="data-table">
								<thead>
									<tr>
										<th onclick={() => toggleBadgePivotSort(false)} class="sortable">Badge{sortIndicator(badgePivotSortByCount ? '' : 'name', badgePivotSortByCount ? '' : 'name', badgePivotSortAsc)}</th>
										<th onclick={() => toggleBadgePivotSort(true)} class="sortable">Scouts{sortIndicator(badgePivotSortByCount ? 'count' : '', badgePivotSortByCount ? 'count' : '', badgePivotSortAsc)}</th>
									</tr>
								</thead>
								<tbody>
									{#each filteredBadgePivot as b}
										<tr
											class:selected={selectedPivotBadge === b.badge_name}
											onclick={() => { selectedPivotBadge = b.badge_name; mobileDetailOpen = true; pivotBadgeReqsScout = null; pivotBadgeReqs = []; pivotBadgeResponse = null; }}
										>
											<td>
												{b.badge_name}
												{#if b.is_eagle_required}<span
														class="eagle-marker"
														title="Eagle required">E</span
													>{/if}
											</td>
											<td>{b.count}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}
					</div>

					<div class="detail-panel" class:mobile-hidden={!mobileDetailOpen}>
						{#if pivotBadgeReqsScout}
							<button class="back-button" onclick={backFromPivotBadgeReqs}>
								&larr; Back
							</button>
							<h2 class="detail-title">{selectedPivotBadge}</h2>
							<p class="detail-subtitle">{pivotBadgeReqsScout.display_name}</p>
							{#if pivotBadgeResponse && pivotBadgeResponse.counselor_name}
								<div class="counselor-info">
									<span class="info-label">Counselor:</span>
									{pivotBadgeResponse.counselor_name}
									{#if pivotBadgeResponse.counselor_phone}
										<span class="text-muted">&middot; {pivotBadgeResponse.counselor_phone}</span>
									{/if}
								</div>
							{/if}
							{#if loadingPivotBadgeReqs}
								<p class="loading-text">Loading requirements...</p>
							{:else if pivotBadgeReqs.length > 0}
								<div class="requirements-list">
									{#each pivotBadgeReqs as req}
										<div class="requirement-item" class:completed={req.is_completed}>
											<span class="req-check">{req.is_completed ? '\u2713' : '\u25CB'}</span>
											<span class="req-number">{req.number}</span>
											<span class="req-text">{req.text}</span>
											{#if req.is_completed}
												<span class="req-date">{req.formatted_date_completed}</span>
											{/if}
										</div>
									{/each}
								</div>
							{:else}
								<p class="empty-message">No requirements data available</p>
							{/if}
						{:else if selectedPivotBadge}
							<h2 class="detail-title">
								{selectedPivotBadge}
								<span class="count-badge">{selectedBadgeScouts.length} scouts</span>
							</h2>
							{#if selectedBadgeScouts.length > 0}
								<table class="detail-table clickable-rows">
									<thead>
										<tr><th>Scout</th><th>Status</th></tr>
									</thead>
									<tbody>
										{#each selectedBadgeScouts as s}
											<tr class="clickable-row" ondblclick={() => viewPivotBadgeRequirements(s)} title="Double-click to view requirements">
												<td>{s.display_name}</td>
												<td>
													{#if s.is_awarded}
														<span class="awarded-text">Awarded {s.formatted_date_awarded}</span>
													{:else if s.is_completed}
														<span class="completed-text">Completed {s.formatted_date_completed}</span>
													{:else if s.percent_completed != null}
														<div class="progress-bar"><div class="progress-fill" style="width: {s.percent_completed}%"></div></div>
														<span class="progress-text">{Math.round(s.percent_completed)}%</span>
													{:else}
														<span class="text-muted">{s.status}</span>
													{/if}
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
							{/if}
						{:else}
							<p class="empty-message">Select a badge to view scouts</p>
						{/if}
					</div>
				</div>

				<!-- ============================================================ -->
				<!-- ADULTS TAB                                                   -->
				<!-- ============================================================ -->
			{:else if activeTab === 'adults'}
				<div class="single-panel">
					<table class="data-table">
						<thead>
							<tr>
								<th
									onclick={() => toggleAdultSort('last_name')}
									class="sortable"
								>
									Name{sortIndicator(
										'last_name',
										adultSortColumn,
										adultSortAsc
									)}
								</th>
								<th
									onclick={() => toggleAdultSort('position')}
									class="sortable"
								>
									Position{sortIndicator(
										'position',
										adultSortColumn,
										adultSortAsc
									)}
								</th>
								<th>Phone</th>
								<th>Email</th>
							</tr>
						</thead>
						<tbody>
							{#each filteredAdults as a}
								<tr>
									<td>{a.display_name}</td>
									<td>{a.role}</td>
									<td>{a.phone || ''}</td>
									<td>{a.email || ''}</td>
								</tr>
							{/each}
							{#if filteredAdults.length === 0}
								<tr
									><td colspan="4" class="empty-message"
										>No adults found</td
									></tr
								>
							{/if}
						</tbody>
					</table>
				</div>

				<!-- ============================================================ -->
				<!-- UNIT TAB                                                     -->
				<!-- ============================================================ -->
			{:else if activeTab === 'unit'}
				<div class="split-view">
					<div class="unit-column">
					{#if unitInfo}
						<section class="info-card">
							<h3>Unit Information</h3>
							<table class="detail-table">
								<tbody>
									<tr><td>Unit</td><td>{unitInfo.name || ''}</td></tr>
									<tr><td>Council</td><td>{unitInfo.council_name || ''}</td></tr>
									<tr><td>District</td><td>{unitInfo.district_name || ''}</td></tr>
									<tr><td>Chartered Org</td><td>{unitInfo.charter_org_name || ''}</td></tr>
									{#if unitInfo.charter_expiry}
										{@const charterDate = new Date(unitInfo.charter_expiry + 'T00:00:00')}
										{@const charterExpired = charterDate < new Date()}
										<tr><td>Charter Status</td><td class={charterExpired ? 'status-expired' : 'status-active'}>{charterExpired ? 'Expired' : 'Expires'} {charterDate.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}</td></tr>
									{/if}
									{#if unitInfo.website}
										<tr><td>Website</td><td class="break-url">{unitInfo.website}</td></tr>
									{/if}
									{#if unitInfo.registration_url}
										<tr><td>Registration</td><td class="break-url">{unitInfo.registration_url}</td></tr>
									{/if}
									{#if unitInfo.meeting_location}
										<tr><td>Meeting Location</td><td>{#if unitInfo.meeting_location.address_line1}{unitInfo.meeting_location.address_line1}{/if}{#if unitInfo.meeting_location.address_line2}, {unitInfo.meeting_location.address_line2}{/if}{#if unitInfo.meeting_location.city || unitInfo.meeting_location.state}<br />{unitInfo.meeting_location.city || ''}{#if unitInfo.meeting_location.state}, {unitInfo.meeting_location.state}{/if} {unitInfo.meeting_location.zip || ''}{/if}</td></tr>
									{/if}
									{#if unitInfo.contacts && unitInfo.contacts.length > 0}
										{#each unitInfo.contacts as c}
											<tr><td>Contact</td><td>{[c.first_name, c.last_name].filter(Boolean).join(' ')}{#if c.email} &middot; <span class="text-muted">{c.email}</span>{/if}{#if c.phone} &middot; <span class="text-muted">{c.phone}</span>{/if}</td></tr>
										{/each}
									{/if}
								</tbody>
							</table>
						</section>
					{/if}

					{#if key3 && (key3.scoutmaster || key3.committee_chair || key3.charter_org_rep)}
						<section class="info-card">
							<h3>Key 3 Leaders</h3>
							<table class="detail-table">
								<thead>
									<tr><th>Position</th><th>Name</th></tr>
								</thead>
								<tbody>
									{#if key3.scoutmaster}
										<tr>
											<td>Scoutmaster</td>
											<td
												>{key3.scoutmaster.first_name}
												{key3.scoutmaster.last_name}</td
											>
										</tr>
									{/if}
									{#if key3.committee_chair}
										<tr>
											<td>Committee Chair</td>
											<td
												>{key3.committee_chair.first_name}
												{key3.committee_chair.last_name}</td
											>
										</tr>
									{/if}
									{#if key3.charter_org_rep}
										<tr>
											<td>Charter Org Rep</td>
											<td
												>{key3.charter_org_rep.first_name}
												{key3.charter_org_rep.last_name}</td
											>
										</tr>
									{/if}
								</tbody>
							</table>
						</section>
					{/if}

					{#if commissioners.length > 0}
						<section class="info-card">
							<h3>Commissioners ({commissioners.length})</h3>
							<table class="detail-table">
								<thead>
									<tr><th>Name</th><th>Position</th></tr>
								</thead>
								<tbody>
									{#each commissioners as c}
										<tr>
											<td
												>{[c.first_name, c.last_name]
													.filter(Boolean)
													.join(' ')}</td
											>
											<td>{c.position || ''}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</section>
					{/if}

					{#if patrols.length > 0}
						<section class="info-card">
							<h3>Patrols ({patrols.length})</h3>
							<div class="patrol-grid">
								{#each patrols as p}
									<div class="patrol-card">
										<span class="patrol-name">{p.subUnitName}</span>
										{#if p.memberCount}<span class="patrol-count">
												({p.memberCount})</span
											>{/if}
										{#if p.patrolLeaderName}<span class="patrol-leader"
												>{p.patrolLeaderName}</span
											>{/if}
									</div>
								{/each}
							</div>
						</section>
					{/if}
					</div>
					<div class="unit-column">
					<!-- Positions of Responsibility -->
					{#if youthPositions.length > 0}
						<section class="info-card">
							<h3>Positions of Responsibility ({youthPositions.length})</h3>
								<table class="detail-table">
									<thead><tr><th>Position</th><th>Scout</th></tr></thead>
									<tbody>
										{#each youthPositions as p}
											<tr><td>{p.display}</td><td>{p.name}</td></tr>
										{/each}
									</tbody>
								</table>
							</section>
					{/if}

					<!-- Membership Renewals -->
					{#if scoutRenewals.length > 0 || adultRenewals.length > 0}
						<section class="info-card">
							<h3>Membership Renewals</h3>
							{#if scoutRenewals.length > 0}
								<h4>Scouts ({scoutRenewals.length} issue{scoutRenewals.length === 1 ? '' : 's'})</h4>
								<table class="detail-table">
									<thead><tr><th>Scout</th><th>Status</th></tr></thead>
									<tbody>
										{#each scoutRenewals as y}
											<tr><td>{y.display_name}</td><td class={y.membership_style === 'expired' ? 'status-expired' : 'status-expiring'}>{y.membership_status}</td></tr>
										{/each}
									</tbody>
								</table>
							{/if}
							{#if adultRenewals.length > 0}
								<h4>Adults ({adultRenewals.length} issue{adultRenewals.length === 1 ? '' : 's'})</h4>
								<table class="detail-table">
									<thead><tr><th>Adult</th><th>Status</th></tr></thead>
									<tbody>
										{#each adultRenewals as a}
											<tr><td>{a.display_name}</td><td class={a.membership_style === 'expired' ? 'status-expired' : 'status-expiring'}>{a.membership_status}</td></tr>
										{/each}
									</tbody>
								</table>
							{/if}
						</section>
					{/if}

					<!-- Training Status -->
					{#if yptIssues.length > 0 || notTrained.length > 0}
						<section class="info-card">
							<h3>Training Status</h3>
							{#if yptIssues.length > 0}
								<h4>Youth Protection ({yptIssues.length} issue{yptIssues.length === 1 ? '' : 's'})</h4>
								<table class="detail-table">
									<thead><tr><th>Adult</th><th>YPT Status</th></tr></thead>
									<tbody>
										{#each yptIssues as a}
											<tr><td>{a.display_name}</td><td class={a.ypt_style === 'expired' ? 'status-expired' : 'status-expiring'}>{a.ypt_status}</td></tr>
										{/each}
									</tbody>
								</table>
							{/if}
							{#if notTrained.length > 0}
								<h4>Position Training ({notTrained.length} not trained)</h4>
								<table class="detail-table">
									<thead><tr><th>Adult</th><th>Status</th></tr></thead>
									<tbody>
										{#each notTrained as a}
											<tr><td>{a.display_name}</td><td class="status-expired">Not Trained</td></tr>
										{/each}
									</tbody>
								</table>
							{/if}
						</section>
					{/if}
					</div>
				</div>
			{/if}
		</main>

		<!-- Bottom Tab Bar (mobile only) -->
		<nav class="bottom-tabs mobile-only">
			{#each tabs as tab}
				<button
					class="bottom-tab"
					class:active={activeTab === tab.id}
					onclick={() => {
						activeTab = tab.id;
						mobileDetailOpen = false;
						selectedYouth = null;
						selectedEvent = null;
						detailView = 'overview';
					}}
				>
					{tab.label}
				</button>
			{/each}
		</nav>

		<!-- Status Bar -->
		<footer class="status-bar">
			<div class="status-left">
				{#if refreshProgress}
					<div class="refresh-bar">
						<div
							class="refresh-fill"
							style="width: {(refreshProgress.current / refreshProgress.total) * 100}%"
						></div>
					</div>
				{/if}
				<span
					class={statusErrors.length > 0 ? 'status-has-errors' : ''}
					title={statusErrors.length > 0 ? statusErrors.join('\n') : ''}
				>{statusMessage}</span>

				{#if activeTab === 'events' && calendarUrl}
					<span class="calendar-url">Subscribe: {calendarUrl}</span>
				{/if}
			</div>
			{#if offlineMode}<span class="offline-indicator">OFFLINE</span>{/if}
			{#if cacheAges}
				<span class="cache-info">
					{#if cacheAges.youth}Roster: {cacheAges.youth}{/if}
					{#if cacheAges.events}&middot; Events: {cacheAges.events}{/if}
					{#if cacheAges.advancement}&middot; Advancement: {cacheAges.advancement}{/if}
					{#if !cacheAges.youth && !cacheAges.events && !cacheAges.advancement}No cache{/if}
				</span>
			{/if}
		</footer>
	</div>
{/if}

<!-- Offline Confirmation Modal -->
{#if showOfflineModal}
	<div class="modal-overlay" role="dialog" aria-modal="true" onclick={() => (showOfflineModal = false)} onkeydown={(e) => { if (e.key === 'Escape') showOfflineModal = false; }} tabindex="-1">
		<div class="login-box modal-box" role="none" onclick={(e) => e.stopPropagation()}>
			{#if !offlineMode}
				<h1 class="login-title">Go Offline?</h1>
				<button class="login-button" onclick={() => { showOfflineModal = false; toggleOfflineMode(); }}>Go Offline</button>
				<button class="login-button modal-cancel" onclick={() => (showOfflineModal = false)}>Cancel</button>
			{:else}
				<h1 class="login-title">Offline Mode</h1>
				<button class="login-button" onclick={() => { showOfflineModal = false; toggleOfflineMode(); }}>Go Online</button>
				<button class="login-button modal-cancel" onclick={() => (showOfflineModal = false)}>Stay Offline</button>
			{/if}
		</div>
	</div>
{/if}

<!-- Quit Confirmation Modal -->
{#if confirmQuit}
	<div class="modal-overlay" role="dialog" aria-modal="true" onclick={() => (confirmQuit = false)} onkeydown={(e) => { if (e.key === 'Escape') confirmQuit = false; }} tabindex="-1">
		<div class="login-box modal-box" role="none" onclick={(e) => e.stopPropagation()}>
			<h1 class="login-title">Quit Trailcache?</h1>
			<button class="login-button" onclick={confirmAndQuit}>Quit</button>
			<button class="login-button modal-cancel" onclick={() => (confirmQuit = false)}>Cancel</button>
		</div>
	</div>
{/if}

<style>
	/* ================================================================ */
	/* Login                                                            */
	/* ================================================================ */

	.login-container {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		background: var(--bg);
	}

	.login-box {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: 12px;
		padding: 2.5rem;
		width: 380px;
		max-width: 90vw;
	}

	.login-title {
		font-size: 1.8rem;
		color: var(--accent);
		text-align: center;
		margin-bottom: 0.25rem;
	}

	.offline-login-note {
		text-align: center;
		color: var(--text-muted);
		font-size: 0.8rem;
		margin-top: -1rem;
		margin-bottom: 1.5rem;
	}

	.login-subtitle {
		text-align: center;
		color: var(--text-muted);
		margin-bottom: 1.5rem;
		font-size: 0.9rem;
	}

	.field-label {
		display: block;
		font-size: 0.85rem;
		color: var(--text-muted);
		margin-bottom: 1rem;
	}

	.field-label input {
		display: block;
		width: 100%;
		margin-top: 0.35rem;
		padding: 0.6rem 0.8rem;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		color: var(--text);
		font-size: 0.95rem;
		outline: none;
		transition: border-color 0.15s;
	}

	.field-label input:focus {
		border-color: var(--accent);
	}

	.error-message {
		background: rgba(247, 118, 142, 0.1);
		border: 1px solid var(--error);
		color: var(--error);
		padding: 0.5rem 0.75rem;
		border-radius: 6px;
		font-size: 0.85rem;
		margin-bottom: 1rem;
	}

	.login-button {
		width: 100%;
		padding: 0.65rem;
		background: var(--accent);
		color: var(--bg);
		border: none;
		border-radius: 6px;
		font-size: 1rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.15s;
	}

	.login-button:hover:not(:disabled) {
		background: var(--accent-hover);
	}

	.login-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* ================================================================ */
	/* App Layout                                                       */
	/* ================================================================ */

	.app-layout {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	.top-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 1rem;
		height: 48px;
		background: var(--bg-surface);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.top-bar-left {
		display: flex;
		align-items: center;
		gap: 1.5rem;
	}

	.top-bar-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.app-name {
		font-weight: 700;
		color: var(--accent);
		font-size: 1.05rem;
	}

	.tab-nav {
		display: flex;
		gap: 2px;
	}

	.tab-button {
		padding: 0.4rem 0.9rem;
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 0.85rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.tab-button:hover {
		color: var(--text);
		background: var(--bg-hover);
	}

	.tab-button.active {
		color: var(--accent);
		background: var(--bg);
	}

	.search-input {
		padding: 0.35rem 0.7rem;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 4px;
		color: var(--text);
		font-size: 0.85rem;
		width: 180px;
		outline: none;
	}

	.search-input:focus {
		border-color: var(--accent);
	}

	.icon-button {
		padding: 0.35rem 0.6rem;
		background: none;
		border: 1px solid var(--border);
		color: var(--text-muted);
		border-radius: 4px;
		cursor: pointer;
		font-size: 1rem;
		transition: all 0.15s;
	}

	.icon-button:hover:not(:disabled) {
		color: var(--text);
		border-color: var(--text-muted);
	}

	.icon-button:disabled {
		opacity: 0.4;
	}

	.icon-button.offline-active {
		color: #ef4444;
		border-color: #ef4444;
	}

	/* ================================================================ */
	/* Modals                                                           */
	/* ================================================================ */

	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal-box.login-box {
		width: 320px;
		padding: 1.75rem 2rem;
		text-align: center;
	}

	.modal-box .login-title {
		font-size: 1.3rem;
		margin-bottom: 0.5rem;
	}

	.modal-box .login-button {
		padding: 0.45rem 1rem;
		font-size: 0.85rem;
	}

	.modal-box .modal-cancel {
		margin-top: 0.4rem;
		background: transparent;
		border-color: var(--border);
		color: var(--text-muted);
	}

	.modal-box .modal-cancel:hover:not(:disabled) {
		background: var(--bg);
		color: var(--text);
	}

	/* ================================================================ */
	/* Status Bar Indicators                                            */
	/* ================================================================ */

	.status-has-errors {
		color: #ef4444;
		cursor: help;
		text-decoration: underline dotted;
	}

	.offline-indicator {
		color: #ef4444;
		font-weight: 700;
		font-size: 0.75rem;
	}

	/* ================================================================ */
	/* Content                                                          */
	/* ================================================================ */

	.content {
		flex: 1;
		overflow: hidden;
	}

	.split-view {
		display: grid;
		grid-template-columns: 1fr 1fr;
		height: 100%;
	}

	.list-panel {
		overflow-y: auto;
		border-right: 1px solid var(--border);
	}

	.detail-panel {
		overflow-y: auto;
		padding: 1rem;
	}

	.single-panel {
		overflow-y: auto;
		height: 100%;
		padding: 0;
	}

	.unit-column {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		overflow-y: auto;
	}

	.panel-message {
		padding: 2rem;
	}

	/* ================================================================ */
	/* Tables                                                           */
	/* ================================================================ */

	.data-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}

	.data-table thead {
		position: sticky;
		top: 0;
		background: var(--bg-surface);
		z-index: 1;
	}

	.data-table th {
		text-align: left;
		padding: 0.5rem 0.75rem;
		color: var(--text-muted);
		font-weight: 600;
		border-bottom: 1px solid var(--border);
		white-space: nowrap;
	}

	.data-table th.sortable {
		cursor: pointer;
		user-select: none;
	}

	.data-table th.sortable:hover {
		color: var(--text);
	}

	.data-table td {
		padding: 0.45rem 0.75rem;
		border-bottom: 1px solid rgba(59, 66, 97, 0.4);
	}

	.data-table tbody tr {
		cursor: pointer;
		transition: background 0.1s;
	}

	.data-table tbody tr:hover {
		background: var(--bg-hover);
	}

	.data-table tbody tr.selected {
		background: rgba(122, 162, 247, 0.15);
	}

	.detail-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.82rem;
	}

	.detail-table th {
		text-align: left;
		padding: 0.35rem 0.5rem;
		color: var(--text-muted);
		font-weight: 600;
		border-bottom: 1px solid var(--border);
	}

	.detail-table td {
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid rgba(59, 66, 97, 0.3);
	}

	.detail-table td.break-url {
		word-break: break-all;
	}

	.unit-column .detail-table td:first-child {
		white-space: nowrap;
		width: 40%;
		color: var(--text-muted);
	}

	.detail-table.clickable-rows tbody tr {
		cursor: pointer;
		transition: background 0.1s;
	}

	.clickable-row:hover {
		background: var(--bg-hover);
	}

	/* ================================================================ */
	/* Detail Panel                                                     */
	/* ================================================================ */

	.detail-title {
		font-size: 1.2rem;
		color: var(--text-bright);
		margin-bottom: 0.5rem;
	}

	.detail-subtitle {
		font-size: 0.85rem;
		color: var(--text-muted);
		margin-bottom: 1rem;
	}

	.detail-meta {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}

	.badge {
		padding: 0.2rem 0.6rem;
		background: rgba(122, 162, 247, 0.15);
		color: var(--accent);
		border-radius: 4px;
		font-size: 0.8rem;
	}

	.badge-muted {
		background: rgba(86, 95, 137, 0.2);
		color: var(--text-muted);
	}

	.count-badge {
		font-size: 0.8rem;
		color: var(--text-muted);
		font-weight: 400;
		margin-left: 0.5rem;
	}

	.detail-section {
		margin-bottom: 1.25rem;
	}

	.detail-section h3 {
		font-size: 0.9rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.section-meta {
		font-size: 0.75rem;
		text-transform: none;
		letter-spacing: normal;
		opacity: 0.7;
		margin-left: 0.5rem;
	}

	.progress-bar {
		display: inline-block;
		width: 60px;
		height: 6px;
		background: var(--bg);
		border-radius: 3px;
		overflow: hidden;
		vertical-align: middle;
	}

	.progress-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 3px;
	}

	.progress-text {
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-left: 0.4rem;
	}

	.completed-text {
		color: var(--warning, #fbbf24);
		font-weight: bold;
		font-size: 0.82rem;
	}

	.awarded-text {
		color: var(--success, #4ade80);
		font-weight: bold;
		font-size: 0.82rem;
	}

	.eagle-marker {
		display: inline-block;
		margin-left: 0.4rem;
		padding: 0 0.3rem;
		background: rgba(187, 154, 247, 0.2);
		color: var(--eagle);
		border-radius: 3px;
		font-size: 0.7rem;
		font-weight: 700;
	}

	.text-muted {
		color: var(--text-muted);
		font-size: 0.82rem;
	}

	.empty-message {
		color: var(--text-muted);
		text-align: center;
		padding: 2rem;
		font-style: italic;
	}

	.loading-text {
		color: var(--text-muted);
		padding: 1rem 0;
	}

	/* ================================================================ */
	/* Back Button / Breadcrumb                                         */
	/* ================================================================ */

	.back-button {
		display: inline-block;
		padding: 0.3rem 0.6rem;
		background: none;
		border: 1px solid var(--border);
		color: var(--text-muted);
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.82rem;
		margin-bottom: 0.75rem;
		transition: all 0.15s;
	}

	.back-button:hover {
		color: var(--text);
		border-color: var(--text-muted);
	}

	/* ================================================================ */
	/* Requirements                                                     */
	/* ================================================================ */

	.requirements-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.requirement-item {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		padding: 0.4rem 0.5rem;
		border-radius: 4px;
		font-size: 0.82rem;
		line-height: 1.4;
	}

	.requirement-item:hover {
		background: var(--bg-hover);
	}

	.requirement-item.completed {
		opacity: 0.7;
	}

	.req-check {
		flex-shrink: 0;
		width: 1.2em;
		text-align: center;
		font-weight: bold;
	}

	.requirement-item.completed .req-check {
		color: var(--success);
	}

	.requirement-item:not(.completed) .req-check {
		color: var(--text-muted);
	}

	.req-number {
		flex-shrink: 0;
		font-weight: 600;
		color: var(--accent);
		min-width: 2em;
	}

	.req-text {
		flex: 1;
		white-space: pre-wrap;
	}

	.req-date {
		flex-shrink: 0;
		color: var(--text-muted);
		font-size: 0.75rem;
		margin-left: auto;
	}

	/* ================================================================ */
	/* Counselor Info                                                    */
	/* ================================================================ */

	.counselor-info {
		padding: 0.5rem 0.75rem;
		background: rgba(122, 162, 247, 0.08);
		border: 1px solid rgba(122, 162, 247, 0.15);
		border-radius: 6px;
		font-size: 0.82rem;
		margin-bottom: 1rem;
	}

	/* ================================================================ */
	/* View Toggle (Event Details/RSVP)                                 */
	/* ================================================================ */

	.view-toggle {
		display: flex;
		gap: 2px;
		margin-bottom: 1rem;
		background: var(--bg);
		border-radius: 6px;
		padding: 2px;
		width: fit-content;
	}

	.toggle-button {
		padding: 0.35rem 0.8rem;
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 0.82rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.toggle-button:hover {
		color: var(--text);
	}

	.toggle-button.active {
		background: var(--bg-surface);
		color: var(--accent);
	}

	/* ================================================================ */
	/* Scout Detail Tabs                                                */
	/* ================================================================ */

	.detail-tabs {
		display: flex;
		gap: 2px;
		margin-bottom: 1rem;
		background: var(--bg);
		border-radius: 6px;
		padding: 2px;
		flex-wrap: wrap;
	}

	.detail-tab {
		padding: 0.35rem 0.7rem;
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 0.8rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.detail-tab:hover {
		color: var(--text);
	}

	.detail-tab.active {
		background: var(--bg-surface);
		color: var(--accent);
	}

	/* ================================================================ */
	/* Event Detail                                                     */
	/* ================================================================ */

	.event-detail {
		margin-top: 0.5rem;
	}

	.event-description {
		margin-top: 1rem;
	}

	.event-description h3 {
		font-size: 0.85rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.5rem;
	}

	.description-text {
		font-size: 0.85rem;
		line-height: 1.5;
		white-space: pre-wrap;
		color: var(--text);
	}

	/* ================================================================ */
	/* RSVP                                                             */
	/* ================================================================ */

	.rsvp-view {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.rsvp-section h3 {
		font-size: 0.9rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.5rem;
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.rsvp-counts {
		font-size: 0.75rem;
		text-transform: none;
		letter-spacing: normal;
		display: flex;
		gap: 0.5rem;
	}

	.rsvp-yes {
		color: var(--success);
	}

	.rsvp-no {
		color: var(--error);
	}

	.rsvp-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.25rem 0.5rem;
		font-size: 0.82rem;
		border-radius: 3px;
	}

	.rsvp-indicator {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.4em;
		height: 1.4em;
		border-radius: 3px;
		font-size: 0.7rem;
		font-weight: 700;
	}

	.rsvp-indicator.yes {
		background: rgba(158, 206, 106, 0.2);
		color: var(--success);
	}

	.rsvp-indicator.no {
		background: rgba(247, 118, 142, 0.2);
		color: var(--error);
	}

	/* ================================================================ */
	/* Unit Info                                                        */
	/* ================================================================ */

	.info-card {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 1rem;
	}

	.info-card h3 {
		font-size: 0.9rem;
		color: var(--accent);
		margin-bottom: 0.75rem;
	}

	.info-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
		min-width: 0;
		overflow: hidden;
	}

	.info-item {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.85rem;
		overflow-wrap: break-word;
		word-break: break-word;
		min-width: 0;
	}

	.info-label {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.info-value {
		font-size: 0.85rem;
		color: var(--text);
	}

	.info-value.membership-active {
		color: var(--success, #4ade80);
	}

	.info-value.membership-expired {
		color: var(--error, #f87171);
	}

	.status-expired {
		color: var(--error, #f87171);
		font-weight: 600;
	}

	.status-expiring {
		color: var(--warning, #fbbf24);
		font-weight: 600;
	}

	.status-active {
		color: var(--success, #4ade80);
	}

	.parent-card {
		margin-bottom: 0.75rem;
		padding: 0.5rem 0;
		border-bottom: 1px solid rgba(59, 66, 97, 0.3);
	}

	.parent-card:last-child {
		border-bottom: none;
	}

	.parent-name {
		font-weight: 600;
		font-size: 0.9rem;
		color: var(--text);
		margin-bottom: 0.4rem;
	}

	.patrol-grid {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.patrol-card {
		padding: 0.4rem 0.8rem;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 4px;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}

	.patrol-name {
		font-size: 0.85rem;
	}

	.patrol-count {
		font-size: 0.82rem;
		color: var(--text-muted);
	}

	.patrol-leader {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	/* ================================================================ */
	/* Status Bar                                                       */
	/* ================================================================ */

	.status-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.3rem 1rem;
		background: var(--bg-surface);
		border-top: 1px solid var(--border);
		font-size: 0.75rem;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.status-left {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
	}

	.refresh-bar {
		width: 60px;
		height: 4px;
		background: var(--bg);
		border-radius: 2px;
		overflow: hidden;
		flex-shrink: 0;
	}

	.refresh-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 2px;
		transition: width 0.2s ease;
	}

	.cache-info {
		opacity: 0.7;
		white-space: nowrap;
	}

	.calendar-url {
		opacity: 0.7;
		font-size: 0.7rem;
		margin-left: 0.5rem;
	}

	/* ================================================================ */
	/* Pull-to-Refresh                                                  */
	/* ================================================================ */

	.pull-indicator {
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		background: var(--bg-surface);
		flex-shrink: 0;
	}

	.pull-text {
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	/* ================================================================ */
	/* Mobile Back Button (in top bar)                                  */
	/* ================================================================ */

	.mobile-back-btn {
		display: none;
		padding: 0.4rem 0.6rem;
		background: none;
		border: none;
		color: var(--accent);
		font-size: 1.2rem;
		cursor: pointer;
	}

	/* ================================================================ */
	/* Bottom Tab Bar (mobile)                                          */
	/* ================================================================ */

	.bottom-tabs {
		display: none;
	}

	.bottom-tab {
		flex: 1;
		padding: 0.6rem 0;
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 0.75rem;
		cursor: pointer;
		text-align: center;
		transition: color 0.15s;
	}

	.bottom-tab.active {
		color: var(--accent);
		font-weight: 600;
	}

	/* ================================================================ */
	/* Desktop-only / Mobile-only utility classes                        */
	/* ================================================================ */

	.desktop-only {
		display: flex;
	}

	.mobile-only {
		display: none;
	}

	/* ================================================================ */
	/* Responsive: Mobile (< 768px)                                     */
	/* ================================================================ */

	@media (max-width: 768px) {
		/* Utility classes */
		.desktop-only {
			display: none !important;
		}

		.mobile-only {
			display: flex !important;
		}

		.mobile-hidden {
			display: none !important;
		}

		.hide-mobile {
			display: none !important;
		}


		.calendar-url {
			display: none !important;
		}

		/* Mobile back button */
		.mobile-back-btn {
			display: block;
		}

		/* Top bar: compact */
		.top-bar {
			padding: 0 0.5rem;
			height: 44px;
		}

		.app-name {
			font-size: 0.95rem;
		}

		.search-input {
			width: 120px;
			font-size: 16px; /* prevents iOS zoom */
		}

		.icon-button {
			padding: 0.5rem 0.7rem;
			min-width: 44px;
			min-height: 44px;
			display: flex;
			align-items: center;
			justify-content: center;
		}

		/* Bottom tab bar */
		.bottom-tabs {
			display: flex;
			background: var(--bg-surface);
			border-top: 1px solid var(--border);
			flex-shrink: 0;
			padding-bottom: env(safe-area-inset-bottom, 0);
		}

		.bottom-tab {
			padding: 0.7rem 0;
			font-size: 0.8rem;
			min-height: 44px;
		}

		/* Split view becomes full-width single column */
		.split-view {
			grid-template-columns: 1fr;
		}

		.list-panel {
			border-right: none;
		}

		.detail-panel {
			padding: 0.75rem;
		}

		/* Larger touch targets on table rows */
		.data-table td {
			padding: 0.65rem 0.75rem;
		}

		.data-table th {
			padding: 0.6rem 0.75rem;
		}

		.detail-table td,
		.detail-table th {
			padding: 0.5rem;
		}

		.clickable-row td {
			padding: 0.65rem 0.5rem;
		}

		/* Requirements: larger touch area */
		.requirement-item {
			padding: 0.55rem 0.5rem;
		}

		/* RSVP items: larger touch area */
		.rsvp-item {
			padding: 0.4rem 0.5rem;
		}

		/* Login form: touch-friendly */
		.field-label input {
			padding: 0.75rem 0.8rem;
			font-size: 16px; /* prevents iOS zoom */
		}

		.login-button {
			padding: 0.8rem;
			min-height: 44px;
		}

		/* Info grid: single column on small screens */
		.info-grid {
			grid-template-columns: 1fr;
		}

		/* Status bar: hide cache details on very small screens */
		.status-bar {
			padding: 0.3rem 0.5rem;
			font-size: 0.7rem;
		}

		/* Back button: larger tap target */
		.back-button {
			padding: 0.5rem 0.8rem;
			min-height: 44px;
		}

		/* View toggle buttons: larger tap target */
		.toggle-button {
			padding: 0.5rem 1rem;
			min-height: 44px;
		}
	}
</style>
