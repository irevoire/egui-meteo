pub(crate) mod lexer;
pub(crate) mod parser;

use miette::{Diagnostic, SourceSpan};

#[derive(Diagnostic, Debug, thiserror::Error)]
#[error("oops")]
pub struct Error {
    // The `Source` that miette will use.
    #[source_code]
    src: String,

    // This will underline/mark the specific code inside the larger
    // snippet context.
    #[label = "This is the highlight"]
    err_span: SourceSpan,

    // You can add as many labels as you want.
    // They'll be rendered sequentially.
    #[label("This is bad")]
    snip2: (usize, usize), // `(usize, usize)` is `Into<SourceSpan>`!

    // Snippets can be optional, by using Option:
    #[label("some text")]
    snip3: Option<SourceSpan>,

    // with or without label text
    #[label]
    snip4: Option<SourceSpan>,
}

/*
data
  |> filter (fun year_data -> yer_data.date.month == "Dec" || yer_data.date.month < "Mar")
  |> split (fun point -> point.date.month == "Aug")
  |> foreach (fun data -> data
    |> map (fun point -> point.temperature)
    |> draw
  )
*/
