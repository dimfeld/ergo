import { formatDistanceToNowStrict } from 'date-fns';

export function relativeTime(t: string) {
  return formatDistanceToNowStrict(new Date(t));
}
