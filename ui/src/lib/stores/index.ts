import { writable, derived } from 'svelte/store';
import type { TaskItem, TopicInfo, CheckItem, StatusData } from '$lib/api/types';
import * as api from '$lib/api/client';

// Core state
export const tasks = writable<TaskItem[]>([]);
export const topics = writable<TopicInfo[]>([]);
export const checks = writable<CheckItem[]>([]);
export const status = writable<StatusData | null>(null);
export const currentTopic = writable<string | null>(null);
export const selectedTaskId = writable<string | null>(null);
export const connectionStatus = writable<'connected' | 'disconnected'>('disconnected');

// Derived stores
export const filteredTasks = derived([tasks, currentTopic], ([$tasks, $currentTopic]) => {
	if (!$currentTopic) return $tasks;
	return $tasks.filter((t) => (t.topics || []).includes($currentTopic));
});

export const tasksByStatus = derived(filteredTasks, ($filteredTasks) => {
	const inProgress = $filteredTasks.filter((t) => t.status === 'in_progress');
	const pending = $filteredTasks.filter((t) => t.status === 'pending' && !t.blocked);
	const blocked = $filteredTasks.filter((t) => t.blocked && t.status !== 'done');
	const done = $filteredTasks
		.filter((t) => t.status === 'done')
		.sort((a, b) => {
			if (!a.completed_at && !b.completed_at) return 0;
			if (!a.completed_at) return 1;
			if (!b.completed_at) return -1;
			return b.completed_at.localeCompare(a.completed_at);
		});
	const backlog = $filteredTasks.filter((t) => t.status === 'backlog');

	return { inProgress, pending, blocked, done, backlog };
});

export const selectedTask = derived([tasks, selectedTaskId], ([$tasks, $selectedTaskId]) => {
	if (!$selectedTaskId) return null;
	return $tasks.find((t) => t.id === $selectedTaskId) || null;
});

// Actions
export async function loadAll() {
	try {
		const [statusData, tasksData, topicsData, checksData] = await Promise.all([
			api.getStatus(),
			api.getTasks(),
			api.getTopics(),
			api.getChecks()
		]);

		status.set(statusData);
		tasks.set(tasksData.tasks);
		topics.set(topicsData.topics);
		currentTopic.set(topicsData.current_topic);
		checks.set(checksData.checks);
		connectionStatus.set('connected');
	} catch (error) {
		console.error('Failed to load data:', error);
		connectionStatus.set('disconnected');
	}
}

export async function loadTasks() {
	try {
		const data = await api.getTasks();
		tasks.set(data.tasks);
	} catch (error) {
		console.error('Failed to load tasks:', error);
	}
}

export async function loadTopics() {
	try {
		const data = await api.getTopics();
		topics.set(data.topics);
		currentTopic.set(data.current_topic);
	} catch (error) {
		console.error('Failed to load topics:', error);
	}
}

export async function loadChecks() {
	try {
		const data = await api.getChecks();
		checks.set(data.checks);
	} catch (error) {
		console.error('Failed to load checks:', error);
	}
}

export async function loadStatus() {
	try {
		const data = await api.getStatus();
		status.set(data);
	} catch (error) {
		console.error('Failed to load status:', error);
	}
}

// Long polling
let lastCounter: number | undefined;
let polling = true;

export async function startPolling() {
	polling = true;
	connectionStatus.set('connected');

	while (polling) {
		try {
			const events = await api.getEvents(lastCounter);
			if (events.changed) {
				await loadAll();
			}
			lastCounter = events.counter;
			connectionStatus.set('connected');
		} catch {
			connectionStatus.set('disconnected');
			await new Promise((r) => setTimeout(r, 2000));
		}
	}
}

export function stopPolling() {
	polling = false;
}
