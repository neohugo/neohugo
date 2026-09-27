//! Port of `resources/page/page_nop.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `page.NopPage` (`*page.nopPage`): returned by `.GetPage` misses in render hooks
//! (`PageInner.GetPage`) — falsy through `IsZero()`. nh-hugolib provides the concrete instance
//! (it must implement `Page`); this module holds the contract.

/// Go type string of the nop page.
pub const NOP_PAGE_TYPE: &str = "*page.nopPage";

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_nop.go (592 lines; 0/131 funcs executed)
//   types: nopPage, nopContentRenderer, (group)
//    L57-59: (p *nopPage) Aliases() []string
//    L61-63: (p *nopPage) Sitemap() config.SitemapConfig
//    L65-67: (p *nopPage) Layout() string
//    L69-71: (p *nopPage) AllTranslations() Pages
//    L73-75: (p *nopPage) LanguagePrefix() string
//    L77-79: (p *nopPage) AlternativeOutputFormats() OutputFormats
//    L81-83: (p *nopPage) BaseFileName() string
//    L85-87: (p *nopPage) BundleType() string
//    L89-91: (p *nopPage) Markup(...any) Markup
//    L93-95: (p *nopPage) Content(context.Context) (any, error)
//    L97-99: (p *nopPage) ContentWithoutSummary(ctx context.Context) (template.HTML, error)
//    L101-103: (p *nopPage) ContentBaseName() string
//    L105-107: (p *nopPage) CurrentSection() Page
//    L109-111: (p *nopPage) Data() any
//    L113-115: (p *nopPage) Date() (t time.Time)
//    L117-119: (p *nopPage) Description() string
//    L121-123: (p *nopPage) RefFrom(argsm map[string]any, source any) (string, error)
//    L125-127: (p *nopPage) RelRefFrom(argsm map[string]any, source any) (string, error)
//    L129-131: (p *nopPage) Dir() string
//    L133-135: (p *nopPage) Draft() bool
//    L137-139: (p *nopPage) Eq(other any) bool
//    L141-143: (p *nopPage) ExpiryDate() (t time.Time)
//    L145-147: (p *nopPage) File() *source.File
//    L149-151: (p *nopPage) FileInfo() hugofs.FileMetaInfo
//    L153-155: (p *nopPage) Filename() string
//    L157-159: (p *nopPage) FirstSection() Page
//    L161-163: (p *nopPage) FuzzyWordCount(context.Context) int
//    L165-167: (p *nopPage) GetPage(ref string) (Page, error)
//    L169-171: (p *nopPage) GetParam(key string) any
//    L173-175: (p *nopPage) GetTerms(taxonomy string) Pages
//    L177-179: (p *nopPage) GitInfo() *source.GitInfo
//    L181-183: (p *nopPage) CodeOwners() []string
//    L185-187: (p *nopPage) HasMenuCurrent(menuID string, me *navigation.MenuEntry) bool
//    L189-191: (p *nopPage) HasShortcode(name string) bool
//    L193-195: (p *nopPage) Hugo() (h neohugo.HugoInfo)
//    L197-199: (p *nopPage) InSection(other any) bool
//    L201-203: (p *nopPage) IsAncestor(other any) bool
//    L205-207: (p *nopPage) IsDescendant(other any) bool
//    L209-211: (p *nopPage) IsDraft() bool
//    L213-215: (p *nopPage) IsHome() bool
//    L217-219: (p *nopPage) IsMenuCurrent(menuID string, inme *navigation.MenuEntry) bool
//    L221-223: (p *nopPage) IsNode() bool
//    L225-227: (p *nopPage) IsPage() bool
//    L229-231: (p *nopPage) IsSection() bool
//    L233-235: (p *nopPage) IsTranslated() bool
//    L237-239: (p *nopPage) Keywords() []string
//    L241-243: (p *nopPage) Kind() string
//    L245-247: (p *nopPage) Lang() string
//    L249-251: (p *nopPage) Language() *langs.Language
//    L253-255: (p *nopPage) Lastmod() (t time.Time)
//    L257-259: (p *nopPage) Len(context.Context) int
//    L261-263: (p *nopPage) LinkTitle() string
//    L265-267: (p *nopPage) LogicalName() string
//    L269-271: (p *nopPage) MediaType() (m media.Type)
//    L273-275: (p *nopPage) Menus() (m navigation.PageMenus)
//    L277-279: (p *nopPage) Name() string
//    L281-283: (p *nopPage) Next() Page
//    L285-287: (p *nopPage) OutputFormats() OutputFormats
//    L289-291: (p *nopPage) Pages() Pages
//    L293-295: (p *nopPage) RegularPages() Pages
//    L297-299: (p *nopPage) RegularPagesRecursive() Pages
//    L301-303: (p *nopPage) Paginate(seq any, options ...any) (*Pager, error)
//    L305-307: (p *nopPage) Paginator(options ...any) (*Pager, error)
//    L309-311: (p *nopPage) Param(key any) (any, error)
//    L313-315: (p *nopPage) Params() maps.Params
//    L317-319: (p *nopPage) Page() Page
//    L321-323: (p *nopPage) Parent() Page
//    L325-327: (p *nopPage) Ancestors() Pages
//    L329-331: (p *nopPage) Path() string
//    L333-335: (p *nopPage) PathInfo() *paths.Path
//    L337-339: (p *nopPage) Permalink() string
//    L341-343: (p *nopPage) Plain(context.Context) string
//    L345-347: (p *nopPage) PlainWords(context.Context) []string
//    L349-351: (p *nopPage) Prev() Page
//    L353-355: (p *nopPage) PublishDate() (t time.Time)
//    L357-359: (p *nopPage) PrevInSection() Page
//    L361-363: (p *nopPage) NextInSection() Page
//    L365-367: (p *nopPage) PrevPage() Page
//    L369-371: (p *nopPage) NextPage() Page
//    L373-375: (p *nopPage) RawContent() string
//    L377-379: (p *nopPage) RenderShortcodes(ctx context.Context) (template.HTML, error)
//    L381-383: (p *nopPage) ReadingTime(context.Context) int
//    L385-387: (p *nopPage) Ref(argsm map[string]any) (string, error)
//    L389-391: (p *nopPage) RelPermalink() string
//    L393-395: (p *nopPage) RelRef(argsm map[string]any) (string, error)
//    L397-399: (p *nopPage) Render(ctx context.Context, layout ...string) (template.HTML, error)
//    L401-403: (p *nopPage) RenderString(ctx context.Context, args ...any) (template.HTML, error)
//    L405-407: (p *nopPage) ResourceType() string
//    L409-411: (p *nopPage) Resources() resource.Resources
//    L413-415: (p *nopPage) Scratch() *maps.Scratch
//    L417-419: (p *nopPage) Store() *maps.Scratch
//    L421-423: (p *nopPage) RelatedKeywords(cfg related.IndexConfig) ([]related.Keyword, error)
//    L425-427: (p *nopPage) Section() string
//    L429-431: (p *nopPage) Sections() Pages
//    L433-435: (p *nopPage) SectionsEntries() []string
//    L437-439: (p *nopPage) SectionsPath() string
//    L441-443: (p *nopPage) Site() Site
//    L445-447: (p *nopPage) Sites() Sites
//    L449-451: (p *nopPage) Slug() string
//    L453-455: (p *nopPage) String() string
//    L457-459: (p *nopPage) Summary(context.Context) template.HTML
//    L461-463: (p *nopPage) TableOfContents(context.Context) template.HTML
//    L465-467: (p *nopPage) Title() string
//    L469-471: (p *nopPage) TranslationBaseName() string
//    L473-475: (p *nopPage) TranslationKey() string
//    L477-479: (p *nopPage) Translations() Pages
//    L481-483: (p *nopPage) Truncated(context.Context) bool
//    L485-487: (p *nopPage) Type() string
//    L489-491: (p *nopPage) URL() string
//    L493-495: (p *nopPage) UniqueID() string
//    L497-499: (p *nopPage) Weight() int
//    L501-503: (p *nopPage) WordCount(context.Context) int
//    L505-507: (p *nopPage) Fragments(context.Context) *tableofcontents.Fragments
//    L509-511: (p *nopPage) HeadingsFiltered(context.Context) tableofcontents.Headings
//    L515-518: (r *nopContentRenderer) ParseAndRenderContent(ctx context.Context, content []byte, renderTOC bool) (converter.ResultRender, error)
//    L520-522: (r *nopContentRenderer) ParseContent(ctx context.Context, content []byte) (converter.ResultParse, bool, error)
//    L524-526: (r *nopContentRenderer) RenderContent(ctx context.Context, content []byte, doc any) (converter.ResultRender, bool, error)
//    L538-540: (c *nopMarkup) Render(context.Context) (Content, error)
//    L542-544: (c *nopMarkup) RenderString(ctx context.Context, args ...any) (template.HTML, error)
//    L546-548: (c *nopMarkup) RenderShortcodes(context.Context) (template.HTML, error)
//    L550-552: (c *nopContent) Plain(context.Context) string
//    L554-556: (c *nopContent) PlainWords(context.Context) []string
//    L558-560: (c *nopContent) WordCount(context.Context) int
//    L562-564: (c *nopContent) FuzzyWordCount(context.Context) int
//    L566-568: (c *nopContent) ReadingTime(context.Context) int
//    L570-572: (c *nopContent) Len(context.Context) int
//    L574-576: (c *nopContent) Content(context.Context) (template.HTML, error)
//    L578-580: (c *nopContent) ContentWithoutSummary(context.Context) (template.HTML, error)
//    L582-584: (c *nopMarkup) Fragments(context.Context) *tableofcontents.Fragments
//    L586-588: (c *nopMarkup) FragmentsHTML(context.Context) template.HTML
//    L590-592: (c *nopContent) Summary(context.Context) (Summary, error)
// ---------------------------------------------------------------------------
