# Known Issues

This document lists known bugs and unexpected behaviors in our project or dependencies.

## 1. Inconsistent Text Node Iteration with kuchiki

**Library**: kuchiki 0.8.1

**Description**: When iterating over HTML text nodes, behavior differs based on iterator creation context and HTML structure.

**Example**: See `src/bin/bug_node_iterator.rs`

**Symptoms**:
- All nodes iterated when iterator used in creation scope
- Only first node (usually in `<head>`) iterated when returned from function
- Issue only occurs with `<head>` element present

**Workaround**: Create and use iterator in same scope, or pass entire `NodeRef`

**Notes**: kuchiki is no longer maintained

## 2. Bad Parsing of Self-Closing Span Tags

**Library**: html5ever 0.26

**Description**: When parsing with `html5ever::parse_document` an XHTML string containing a self-closing span tag `<span .. />`, the parser interprets it as an opening tag `<span>` and tries to find a closing tag `</span>`. Because it does not exist, it considers that the span tag closes right before its parent. For example, `<h2><span/>Hi </h2>` becomes `<h2><span>Hi </span></h2>`.

This produces several problems when manipulating XHTML files. Here are two that I have found:

1. Figure tags containing a span become invalid.
   ```html
   <figure>
       <span id="pg1"/>
       <img/>
       <figcaption>
           [Figure caption]
       </figcaption>
   </figure>
   ```
   Becomes:
   ```html
   <figure>
       <span id="pg1">
           <img/>
           <figcaption>
               [Figure caption]
           </figcaption>
       </span>
   </figure>
   ```
   The problem here is that `<figcaption>` can only be inside `<figure>`, so the file becomes invalid.
   Error from epubcheck:
   ```
   ERROR(RSC-005): file.xhtml(x,y): Error while parsing file: element "figcaption" not allowed here;...
   ```

2. Span tags used as references become invalid.
   In EPUB files, empty span tags are sometimes used as references in the index, TOC, or permissions. So if there is a tag like this:
   ```html
   <a href="chapter001.xhtml#pg10">
   ```
   This link points to a span tag like this:
   ```html
   <span id="pg22"/>
   ```
   Which becomes:
   ```html
   <span id="pg22">.. </span>
   ```
   So the link becomes invalid.
   Epubcheck message:
   ```
   ERROR(RSC-012): permissions.xhtml(x,y): Fragment identifier is not defined.
   ```

**Workaround**: Before parsing the document, replace `<span .. />` with `<span ..></span>` using this:
```rust
use regex::Regex;

let re = Regex::new(r"<span([^>]*?)/>")?;
let content = re.replace_all(&content, "<span$1></span>");
```
When serializing, do the opposite by replacing empty span double tags with self-closing ones:
```rust
let re_span = Regex::new(r"<span([^>]*?)></span>")?;
output_string = re_span.replace_all(&output_string, "<span$1/>").to_string();
```

## 3. Bad Serialization of Non-Breaking Spaces

**Library**: html5ever 0.26

**Description**: When serializing XHTML, non-breaking spaces are serialized as `&nbsp;` literal. For example, 3 non-breaking spaces are serialized as `&nbsp;&nbsp;&nbsp;` instead of `   `.

Epubcheck message:
```
FATAL(RSC-016): file.xhtml(x,y): Fatal Error while parsing file: The entity "nbsp" was referenced, but not declared.
```

**Workaround**: Pattern match `&nbsp;` and replace it by `\u{00A0}` with: 
```rust
let re_nbsp = Regex::new(r"&nbsp;")?;
let content = re_nbsp.replace_all(&content, "\u{00A0}").to_string();
```

## 4. Self-closing tags serialized without proper XHTML format

**Library**: html5ever 0.26

**Description**: When serializing XHTML, self closing tags like `br`, `hr`, `img`, `input`, `link`, `meta`, get serialized as `<tag [attributes]>` instead of XHTML-compliant `<tag [attributes] />`. While this output is valid XML, it does not meet the stricter XHTML requirements used in EPUB format.

**Workaround**: Pattern match.
```rust
// Change `>` to `/>` in self closing tags
let self_closing_tags = ["br", "hr", "img", "input", "link", "meta"];
for tag in self_closing_tags.iter() {
    let re = Regex::new(&format!(r"(<{}[^>]*?)>", tag).to_string())?;
    output_string = re.replace_all(&output_string, "$1/>").to_string();
}
```

## 5. Whitespace trimming in `em` and `a` tags causes word juxtaposition

**Library**: html5ever 0.26

**Description**: During the translation process, the content of these tags, often a single word, get's trimmed. 

**Example** The original text `what’s <em>morally</em>` might become `es<em>moralmente</em>`. which would be rendered as `esmoralmente` in the e-reader.

**Workaround**: Use regex pattern matching to ensure proper spacing around these tags.
```rust
 let re_em = Regex::new(r"([^>\s])(</?em>)([^<\s])")?;
output_string = re_em.replace_all(&output_string, "$1 $2$3").to_string();

let re_a = Regex::new(r"([^>\s])(</?a [^>]*?>)([^<\s])")?;
output_string = re_a.replace_all(&output_string, "$1 $2$3").to_string();
``` 
