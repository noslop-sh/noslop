import type { DiffLineAnnotation } from '@pierre/diffs';
import type { Finding } from '$lib/types';
import type { AnnotationMeta } from './types';

/**
 * Build annotation descriptors for a single file from a list of findings.
 * Pure function — no closures over component state.
 */
export function buildAnnotationsForFile(
  findings: Finding[],
  filePath: string
): DiffLineAnnotation<AnnotationMeta>[] {
  return findings
    .filter((f) => f.target.path === filePath && f.target.span !== null)
    .map((f) => ({
      side: 'additions' as const,
      lineNumber: f.target.span!.start,
      metadata: { finding: f },
    }));
}
