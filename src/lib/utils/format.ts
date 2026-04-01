/**
 * 格式化总秒数为 MM:SS
 */
export function formatTime(totalSeconds: number): string {
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}

/**
 * 格式化 ISO 时间戳为完整日期时间字符串
 */
export function formatTimestamp(ts: string): string {
  try {
    const d = new Date(ts);
    const pad = (n: number) => n.toString().padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  } catch {
    return ts;
  }
}

/**
 * 格式化 ISO 日期字符串为短日期 (MM/DD)
 */
export function formatDate(isoStr: string): string {
  try {
    return new Date(isoStr).toLocaleDateString(undefined, {
      month: '2-digit',
      day: '2-digit'
    });
  } catch {
    return '';
  }
}
