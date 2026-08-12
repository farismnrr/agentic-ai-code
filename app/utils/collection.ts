export function replaceById<T extends { id: string }>(items: T[], id: string, value: T): T[] {
  return items.map(item => item.id === id ? value : item)
}

export function removeById<T extends { id: string }>(items: T[], id: string): T[] {
  return items.filter(item => item.id !== id)
}
