import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock fetch globally
const mockFetch = vi.fn();
globalThis.fetch = mockFetch as typeof fetch;

// Import after mocking
import * as api from './client';

describe('API Client', () => {
	beforeEach(() => {
		mockFetch.mockReset();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('getStatus', () => {
		it('should fetch status successfully', async () => {
			const mockData = {
				branch: 'main',
				current_task: 'TSK-1',
				tasks: { total: 5, backlog: 1, pending: 2, in_progress: 1, done: 1 },
				checks: 3
			};

			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: mockData })
			});

			const result = await api.getStatus();

			expect(mockFetch).toHaveBeenCalledWith('/api/v1/status', expect.any(Object));
			expect(result).toEqual(mockData);
		});

		it('should throw on API error', async () => {
			mockFetch.mockResolvedValueOnce({
				json: () =>
					Promise.resolve({
						success: false,
						error: { code: 'NOT_FOUND', message: 'Status not found' }
					})
			});

			await expect(api.getStatus()).rejects.toThrow('Status not found');
		});
	});

	describe('getTasks', () => {
		it('should fetch tasks successfully', async () => {
			const mockData = {
				tasks: [
					{
						id: 'TSK-1',
						title: 'Test task',
						status: 'pending',
						priority: 'p2',
						current: false,
						blocked: false,
						check_count: 0,
						checks_verified: 0
					}
				]
			};

			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: mockData })
			});

			const result = await api.getTasks();

			expect(mockFetch).toHaveBeenCalledWith('/api/v1/tasks', expect.any(Object));
			expect(result.tasks).toHaveLength(1);
			expect(result.tasks[0].id).toBe('TSK-1');
		});
	});

	describe('createTask', () => {
		it('should create task with title', async () => {
			const mockResponse = {
				id: 'TSK-2',
				title: 'New task',
				status: 'pending',
				priority: 'p2'
			};

			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: mockResponse })
			});

			const result = await api.createTask({ title: 'New task' });

			expect(mockFetch).toHaveBeenCalledWith(
				'/api/v1/tasks',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify({ title: 'New task' })
				})
			);
			expect(result.id).toBe('TSK-2');
		});
	});

	describe('updateTask', () => {
		it('should update task fields', async () => {
			const mockResponse = {
				id: 'TSK-1',
				title: 'Updated title',
				description: 'New description',
				status: 'pending',
				priority: 'p2',
				current: false,
				blocked: false,
				check_count: 0,
				checks_verified: 0,
				created_at: '2024-01-01T00:00:00Z',
				checks: []
			};

			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: mockResponse })
			});

			const result = await api.updateTask('TSK-1', {
				title: 'Updated title',
				description: 'New description'
			});

			expect(mockFetch).toHaveBeenCalledWith(
				'/api/v1/tasks/TSK-1',
				expect.objectContaining({
					method: 'PATCH'
				})
			);
			expect(result.title).toBe('Updated title');
		});
	});

	describe('startTask', () => {
		it('should start a task', async () => {
			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: { id: 'TSK-1', status: 'in_progress' } })
			});

			const result = await api.startTask('TSK-1');

			expect(mockFetch).toHaveBeenCalledWith(
				'/api/v1/tasks/TSK-1/start',
				expect.objectContaining({ method: 'POST' })
			);
			expect(result.status).toBe('in_progress');
		});
	});

	describe('completeTask', () => {
		it('should complete a task', async () => {
			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: { id: 'TSK-1', status: 'done' } })
			});

			const result = await api.completeTask('TSK-1');

			expect(mockFetch).toHaveBeenCalledWith(
				'/api/v1/tasks/TSK-1/done',
				expect.objectContaining({ method: 'POST' })
			);
			expect(result.status).toBe('done');
		});
	});

	describe('getConcepts', () => {
		it('should fetch concepts', async () => {
			const mockData = {
				concepts: [{ id: 'CON-1', name: 'Auth', task_count: 3, created_at: '2024-01-01' }],
				current_concept: null
			};

			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: mockData })
			});

			const result = await api.getConcepts();

			expect(result.concepts).toHaveLength(1);
			expect(result.concepts[0].name).toBe('Auth');
		});
	});

	describe('getEvents', () => {
		it('should poll for events without since parameter', async () => {
			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: { changed: false, counter: 5 } })
			});

			const result = await api.getEvents();

			expect(mockFetch).toHaveBeenCalledWith('/api/v1/events', expect.any(Object));
			expect(result.counter).toBe(5);
		});

		it('should poll for events with since parameter', async () => {
			mockFetch.mockResolvedValueOnce({
				json: () => Promise.resolve({ success: true, data: { changed: true, counter: 10 } })
			});

			const result = await api.getEvents(5);

			expect(mockFetch).toHaveBeenCalledWith('/api/v1/events?since=5', expect.any(Object));
			expect(result.changed).toBe(true);
		});
	});
});
