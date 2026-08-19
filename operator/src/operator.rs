/*
The CLI Payload Shape: Have your CLI tool package the input into a clean, delimited format. For example, if the operator types an option, the CLI ships a flat string over the WebSocket wire like: COMMAND:argument_1:argument_
2.The Server String Parser: On the server side, when the WebSocket stream receives that byte buffer, convert it to a string slice (&str) and use Rust’s built-in, zero-copy string splitters (like .split(':')) to break the message apart into an array.
The Weekend Unit Tests: Write unit tests that throw edge cases at your parser. Test what happens if the user sends an empty string, an infinite string of colons (::::), or non-UTF8 characters. Proving your server discards these safely without a runtime panic (.unwrap()) is how you guarantee you won't get smited.
*/