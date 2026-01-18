import type { Snippet } from 'svelte';
import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

// Type utility for components that forward element refs
export type WithElementRef<T, E extends HTMLElement = HTMLElement> = T & {
	ref?: E | null;
};

// Type utilities for shadcn-svelte components
export type WithoutChild<T> = T extends { child?: Snippet } ? Omit<T, 'child'> : T;
export type WithoutChildren<T> = T extends { children?: Snippet } ? Omit<T, 'children'> : T;
export type WithoutChildrenOrChild<T> = WithoutChild<WithoutChildren<T>>;
