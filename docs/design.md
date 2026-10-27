# Schéma visual identity

The canonical application mark is copied unchanged from `scnq-schema/Main/gamma/Assets/Logos/scenic-tools-schema.ico` to `web/favicon.ico`. It appears in the header and browser tab.

The icon's dominant opaque RGB is `(114, 222, 194)`, **#72DEC2**. Both themes use this mint teal as `--accent`; light-mode focus outlines use deeper teal **#18745F** for contrast. Dark mode uses neutral charcoal **#14191B**, with raised surfaces and mint-tinted selections. Light mode uses cool off-white **#F5F8F7**. Block icons share the mint accent, with restrained violet/blue details to distinguish operations.

Theme tokens live in `web/style.css`; saved/system theme handling remains in `web/app.js`. The read-only browser checks in `tests/theme.cjs` verify the brand token, original mark, dark surface, theme toggle, persistence and system preference. Catalog screenshots show both themes.
