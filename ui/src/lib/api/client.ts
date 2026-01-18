// API Client for noslop backend

import type {
	ApiResponse,
	StatusData,
	TasksData,
	TaskDetailData,
	TaskCreateData,
	TaskMutationData,
	ChecksData,
	CheckCreateData,
	ConceptsData,
	ConceptInfo,
	ConceptCreateData,
	EventsData,
	CreateTaskRequest,
	CreateCheckRequest,
	CreateConceptRequest,
	UpdateTaskRequest,
	UpdateConceptRequest,
	BlockerRequest,
	LinkBranchRequest,
	SelectConceptRequest
} from './types';

const API_BASE = '/api/v1';

async function request<T>(method: string, endpoint: string, body?: unknown): Promise<T> {
	const options: RequestInit = {
		method,
		headers: {
			'Content-Type': 'application/json'
		}
	};

	if (body) {
		options.body = JSON.stringify(body);
	}

	const response = await fetch(`${API_BASE}${endpoint}`, options);
	const envelope: ApiResponse<T> = await response.json();

	if (!envelope.success) {
		throw new Error(envelope.error?.message || 'API request failed');
	}

	return envelope.data as T;
}

// Status
export async function getStatus(): Promise<StatusData> {
	return request<StatusData>('GET', '/status');
}

// Tasks
export async function getTasks(): Promise<TasksData> {
	return request<TasksData>('GET', '/tasks');
}

export async function getTask(id: string): Promise<TaskDetailData> {
	return request<TaskDetailData>('GET', `/tasks/${id}`);
}

export async function createTask(req: CreateTaskRequest): Promise<TaskCreateData> {
	return request<TaskCreateData>('POST', '/tasks', req);
}

export async function updateTask(id: string, req: UpdateTaskRequest): Promise<TaskDetailData> {
	return request<TaskDetailData>('PATCH', `/tasks/${id}`, req);
}

export async function deleteTask(id: string): Promise<TaskMutationData> {
	return request<TaskMutationData>('DELETE', `/tasks/${id}`);
}

export async function startTask(id: string): Promise<TaskMutationData> {
	return request<TaskMutationData>('POST', `/tasks/${id}/start`);
}

export async function completeTask(id: string): Promise<TaskMutationData> {
	return request<TaskMutationData>('POST', `/tasks/${id}/done`);
}

export async function resetTask(id: string): Promise<TaskMutationData> {
	return request<TaskMutationData>('POST', `/tasks/${id}/reset`);
}

export async function backlogTask(id: string): Promise<TaskMutationData> {
	return request<TaskMutationData>('POST', `/tasks/${id}/backlog`);
}

export async function addBlocker(id: string, req: BlockerRequest): Promise<TaskMutationData> {
	return request<TaskMutationData>('POST', `/tasks/${id}/block`, req);
}

export async function removeBlocker(id: string, req: BlockerRequest): Promise<TaskMutationData> {
	return request<TaskMutationData>('POST', `/tasks/${id}/unblock`, req);
}

export async function linkBranch(id: string, req: LinkBranchRequest): Promise<TaskMutationData> {
	return request<TaskMutationData>('POST', `/tasks/${id}/link-branch`, req);
}

// Checks
export async function getChecks(): Promise<ChecksData> {
	return request<ChecksData>('GET', '/checks');
}

export async function createCheck(req: CreateCheckRequest): Promise<CheckCreateData> {
	return request<CheckCreateData>('POST', '/checks', req);
}

// Concepts
export async function getConcepts(): Promise<ConceptsData> {
	return request<ConceptsData>('GET', '/concepts');
}

export async function createConcept(req: CreateConceptRequest): Promise<ConceptCreateData> {
	return request<ConceptCreateData>('POST', '/concepts', req);
}

export async function updateConcept(id: string, req: UpdateConceptRequest): Promise<ConceptInfo> {
	return request<ConceptInfo>('PATCH', `/concepts/${id}`, req);
}

export async function deleteConcept(id: string): Promise<void> {
	return request<void>('DELETE', `/concepts/${id}`);
}

export async function selectConcept(req: SelectConceptRequest): Promise<ConceptsData> {
	return request<ConceptsData>('POST', '/concepts/select', req);
}

// Events (long-polling)
export async function getEvents(since?: number): Promise<EventsData> {
	const url = since !== undefined ? `/events?since=${since}` : '/events';
	return request<EventsData>('GET', url);
}
