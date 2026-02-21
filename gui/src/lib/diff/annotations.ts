import type { DiffLineAnnotation } from '@pierre/diffs';
import type { Finding } from '$lib/types';
import type { AnnotationMeta } from './types';

/**
 * Build annotation descriptors for a single file from a list of findings.
 * Pure function — no closures over component state.
 *
 * For delete-only files (fileChangeType === 'deleted'), annotations target
 * the deletions side since there is no additions column.
 */
export function buildAnnotationsForFile(
  findings: Finding[],
  filePath: string,
  fileChangeType?: string
): DiffLineAnnotation<AnnotationMeta>[] {
  const side: 'deletions' | 'additions' = fileChangeType === 'deleted' ? 'deletions' : 'additions';
  return findings
    .filter((f) => f.target.path === filePath && f.target.span !== null)
    .map((f) => ({
      side,
      lineNumber: f.target.span!.start,
      metadata: { finding: f },
    }));
}
