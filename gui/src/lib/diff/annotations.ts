import type { DiffLineAnnotation } from '@pierre/diffs';
import type { Feedback } from '$lib/types';
import type { AnnotationMeta } from './types';

/**
 * Build annotation descriptors for a single file from a list of feedbacks.
 * Pure function — no closures over component state.
 *
 * For delete-only files (fileChangeType === 'deleted'), annotations target
 * the deletions side since there is no additions column.
 */
export function buildAnnotationsForFile(
  feedbacks: Feedback[],
  filePath: string,
  fileChangeType?: string
): DiffLineAnnotation<AnnotationMeta>[] {
  const side: 'deletions' | 'additions' = fileChangeType === 'deleted' ? 'deletions' : 'additions';
  return feedbacks
    .filter((f) => f.target.path === filePath && f.target.span !== null)
    .map((f) => ({
      side,
      lineNumber: f.target.span!.start,
      metadata: { feedback: f },
    }));
}
