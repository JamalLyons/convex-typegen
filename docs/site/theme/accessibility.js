// mdBook uses a native checkbox with a separate toolbar label for the sidebar.
// Expose the checkbox's native checked state and keep the label presentational.
const sidebarControl = document.querySelector('#mdbook-sidebar-toggle-anchor');
const sidebarLabel = document.querySelector('#mdbook-sidebar-toggle');
const sidebarPanel = document.querySelector('#mdbook-sidebar');

if (sidebarControl && sidebarLabel instanceof HTMLLabelElement && sidebarPanel) {
  sidebarControl.setAttribute('aria-label', 'Show table of contents');
  sidebarControl.setAttribute('aria-controls', sidebarPanel.id);
  sidebarLabel.removeAttribute('aria-label');
  sidebarLabel.removeAttribute('aria-controls');

  const clearExpanded = () => sidebarLabel.removeAttribute('aria-expanded');
  clearExpanded();
  new MutationObserver(clearExpanded).observe(sidebarLabel, {
    attributes: true,
    attributeFilter: ['aria-expanded'],
  });

  // Keyboard activation of the checkbox does not click the toolbar label.
  sidebarControl.addEventListener('change', () => {
    if (sidebarControl.checked) sidebarPanel.style.display = '';
  });
}

// Let keyboard users reach and scroll code and tables that overflow.
const scrollRegions = new ResizeObserver((entries) => {
  for (const { target } of entries) {
    if (target.scrollWidth > target.clientWidth) {
      target.tabIndex = 0;
    } else {
      target.removeAttribute('tabindex');
    }
  }
});

document.querySelectorAll('main pre > code, main .table-wrapper').forEach((region) => {
  scrollRegions.observe(region);
});
