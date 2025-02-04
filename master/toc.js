// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="introduction.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="quickstart.html"><strong aria-hidden="true">2.</strong> Quickstart</a></li><li class="chapter-item expanded "><a href="types/index.html"><strong aria-hidden="true">3.</strong> Type system</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="types/objects/index.html"><strong aria-hidden="true">3.1.</strong> Objects</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="types/objects/complex_fields.html"><strong aria-hidden="true">3.1.1.</strong> Complex fields</a></li><li class="chapter-item expanded "><a href="types/objects/context.html"><strong aria-hidden="true">3.1.2.</strong> Context</a></li><li class="chapter-item expanded "><a href="types/objects/error/index.html"><strong aria-hidden="true">3.1.3.</strong> Error handling</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="types/objects/error/field.html"><strong aria-hidden="true">3.1.3.1.</strong> Field errors</a></li><li class="chapter-item expanded "><a href="types/objects/error/schema.html"><strong aria-hidden="true">3.1.3.2.</strong> Schema errors</a></li></ol></li><li class="chapter-item expanded "><a href="types/objects/generics.html"><strong aria-hidden="true">3.1.4.</strong> Generics</a></li></ol></li><li class="chapter-item expanded "><a href="types/interfaces.html"><strong aria-hidden="true">3.2.</strong> Interfaces</a></li><li class="chapter-item expanded "><a href="types/unions.html"><strong aria-hidden="true">3.3.</strong> Unions</a></li><li class="chapter-item expanded "><a href="types/enums.html"><strong aria-hidden="true">3.4.</strong> Enums</a></li><li class="chapter-item expanded "><a href="types/input_objects.html"><strong aria-hidden="true">3.5.</strong> Input objects</a></li><li class="chapter-item expanded "><a href="types/scalars.html"><strong aria-hidden="true">3.6.</strong> Scalars</a></li></ol></li><li class="chapter-item expanded "><a href="schema/index.html"><strong aria-hidden="true">4.</strong> Schema</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="schema/subscriptions.html"><strong aria-hidden="true">4.1.</strong> Subscriptions</a></li><li class="chapter-item expanded "><a href="schema/introspection.html"><strong aria-hidden="true">4.2.</strong> Introspection</a></li></ol></li><li class="chapter-item expanded "><a href="serve/index.html"><strong aria-hidden="true">5.</strong> Serving</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="serve/batching.html"><strong aria-hidden="true">5.1.</strong> Batching</a></li></ol></li><li class="chapter-item expanded "><a href="advanced/index.html"><strong aria-hidden="true">6.</strong> Advanced Topics</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="advanced/implicit_and_explicit_null.html"><strong aria-hidden="true">6.1.</strong> Implicit and explicit null</a></li><li class="chapter-item expanded "><a href="advanced/n_plus_1.html"><strong aria-hidden="true">6.2.</strong> N+1 problem</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="advanced/dataloader.html"><strong aria-hidden="true">6.2.1.</strong> DataLoader</a></li><li class="chapter-item expanded "><a href="advanced/lookahead.html"><strong aria-hidden="true">6.2.2.</strong> Look-ahead</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="advanced/eager_loading.html"><strong aria-hidden="true">6.2.2.1.</strong> Eager loading</a></li></ol></li></ol></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
