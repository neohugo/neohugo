// Executed as a template first (execute_as_template): the data of the first execution wins.
interface SearchItem { title: string; url: string }
const api: string = "{{ data.api }}";
export function search(items: SearchItem[], q: string): SearchItem[] {
  return items.filter((i) => i.title.toLowerCase().includes(q.toLowerCase()));
}
export const endpoint = `${api}/search`;
