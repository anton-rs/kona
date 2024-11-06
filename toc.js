// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
var sidebarScrollbox = document.querySelector("#sidebar .sidebar-scrollbox");
sidebarScrollbox.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="intro.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="fpp-dev/intro.html"><strong aria-hidden="true">2.</strong> Fault Proof Program Development</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="fpp-dev/env.html"><strong aria-hidden="true">2.1.</strong> Environment</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="fpp-dev/targets.html"><strong aria-hidden="true">2.1.1.</strong> Supported Targets</a></li></ol></li><li class="chapter-item expanded "><a href="fpp-dev/prologue.html"><strong aria-hidden="true">2.2.</strong> Prologue</a></li><li class="chapter-item expanded "><a href="fpp-dev/execution.html"><strong aria-hidden="true">2.3.</strong> Execution</a></li><li class="chapter-item expanded "><a href="fpp-dev/epilogue.html"><strong aria-hidden="true">2.4.</strong> Epilogue</a></li></ol></li><li class="chapter-item expanded "><a href="sdk/intro.html"><strong aria-hidden="true">3.</strong> Kona SDK</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="sdk/fpvm-backend.html"><strong aria-hidden="true">3.1.</strong> FPVM Backend</a></li><li class="chapter-item expanded "><a href="sdk/custom-backend.html"><strong aria-hidden="true">3.2.</strong> Custom Backend</a></li><li class="chapter-item expanded "><a href="sdk/exec-ext.html"><strong aria-hidden="true">3.3.</strong> kona-executor Extensions</a></li><li class="chapter-item expanded "><a href="sdk/pipeline/intro.html"><strong aria-hidden="true">3.4.</strong> kona-derive Pipeline</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="sdk/pipeline/providers.html"><strong aria-hidden="true">3.4.1.</strong> Custom Providers</a></li><li class="chapter-item expanded "><a href="sdk/pipeline/stages.html"><strong aria-hidden="true">3.4.2.</strong> Stage Swapping</a></li><li class="chapter-item expanded "><a href="sdk/pipeline/signaling.html"><strong aria-hidden="true">3.4.3.</strong> Signaling</a></li></ol></li></ol></li><li class="chapter-item expanded "><a href="glossary.html"><strong aria-hidden="true">4.</strong> Glossary</a></li><li class="chapter-item expanded "><a href="CONTRIBUTING.html"><strong aria-hidden="true">5.</strong> Contributing</a></li></ol>';
(function() {
    let current_page = document.location.href.toString();
    if (current_page.endsWith("/")) {
        current_page += "index.html";
    }
    var links = sidebarScrollbox.querySelectorAll("a");
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
})();

// Track and set sidebar scroll position
sidebarScrollbox.addEventListener('click', function(e) {
    if (e.target.tagName === 'A') {
        sessionStorage.setItem('sidebar-scroll', sidebarScrollbox.scrollTop);
    }
}, { passive: true });
var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
sessionStorage.removeItem('sidebar-scroll');
if (sidebarScrollTop) {
    // preserve sidebar scroll position when navigating via links within sidebar
    sidebarScrollbox.scrollTop = sidebarScrollTop;
} else {
    // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
    var activeSection = document.querySelector('#sidebar .active');
    if (activeSection) {
        activeSection.scrollIntoView({ block: 'center' });
    }
}
