// API Response Types - matching Rust API

export interface ApiResponse<T> {
	success: boolean;
	data?: T;
	error?: {
		code: string;
		message: string;
	};
}

export interface StatusData {
	branch: string | null;
	current_task: string | null;
	tasks: TaskCounts;
	checks: number;
}

export interface TaskCounts {
	total: number;
	backlog: number;
	pending: number;
	in_progress: number;
	done: number;
}

export interface TaskItem {
	id: string;
	title: string;
	description?: string;
	status: 'backlog' | 'pending' | 'in_progress' | 'done';
	priority: string;
	blocked_by?: string[];
	current: boolean;
	blocked: boolean;
	branch?: string;
	started_at?: string;
	completed_at?: string;
	topics?: string[];
	scope?: string[];
	check_count: number;
	checks_verified: number;
}

export interface TaskDetailData extends TaskItem {
	created_at: string;
	notes?: string;
	checks: TaskCheckItem[];
}

export interface TaskCheckItem {
	id: string;
	message: string;
	severity: 'block' | 'warn' | 'info';
	verified: boolean;
}

export interface TasksData {
	tasks: TaskItem[];
}

export interface TaskCreateData {
	id: string;
	title: string;
	status: string;
	priority: string;
}

export interface TaskMutationData {
	id: string;
	status: string;
}

export interface CheckItem {
	id: string;
	scope: string;
	message: string;
	severity: 'block' | 'warn' | 'info';
}

export interface ChecksData {
	checks: CheckItem[];
}

export interface CheckCreateData {
	id: string;
	scope: string;
	message: string;
	severity: string;
}

export interface TopicInfo {
	id: string;
	name: string;
	description?: string;
	scope?: string[];
	task_count: number;
	created_at: string;
}

export interface TopicsData {
	topics: TopicInfo[];
	current_topic: string | null;
}

export interface TopicCreateData {
	id: string;
	name: string;
}

export interface EventsData {
	changed: boolean;
	counter: number;
}

// Request types
export interface CreateTaskRequest {
	title: string;
	description?: string;
	priority?: string;
	topics?: string[];
}

export interface CreateCheckRequest {
	scope: string;
	message: string;
	severity?: string;
}

export interface CreateTopicRequest {
	name: string;
	description?: string;
}

export interface UpdateTaskRequest {
	title?: string;
	description?: string | null;
	topics?: string[];
}

export interface UpdateTopicRequest {
	name?: string;
	description?: string | null;
}

export interface BlockerRequest {
	blocker_id: string;
}

export interface LinkBranchRequest {
	branch: string | null;
}

export interface SelectTopicRequest {
	id: string | null;
}
