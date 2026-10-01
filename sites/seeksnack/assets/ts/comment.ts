const lang: string = "{{ data.lang }}";
export function commentURL(id: string): string {
  return `${endpoint}/comments/${id}?lang=${lang}`;
}
console.log(commentURL("x"));
