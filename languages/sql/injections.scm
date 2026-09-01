((comment) @injection.content
  (#set! injection.language "comment"))

((marginalia) @injection.content
  (#set! injection.language "comment"))

((ERROR) @injection.content
  (#match? @injection.content "^{{|{%|{#")
  (#set! injection.language "jinja2"))
