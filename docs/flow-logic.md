# Flow scalar line joining

## Forward

These are notes are from my investigation of how to
do line joins in flow scalars. They are preserved
for posterity, but are largely incorrect

## Notes


```rust
leading_blanks->false
string->String::new()
whitespaces->String::new()
trailing_breaks->String::new()
leading_break->String::new()

while parser[0]->!isBlankZ!
    // <...>
    if scalar_double && parser[0]->'\\' && parser[1]->'\n' // if double quoted and escaped newline
        leading_blanks = true
        break
    // <...>

while parser[0]->isBlank! || parser[0]->isBreak!
    if isBlank!:                                      // current byte is a space or newline
        if leading_blanks:
            parser->skip(1)
        else
            parser->read(1)->whitespaces
    else if isBreak!:                                 // current byte is a yaml line break
        if leading_blanks:                            // this looks like an initialization clause, either
            parser->read_line(1)->trailing_breaks     // A. we are already in state "leading_blanks", in which case append to trailing_breaks
        else                                          // B. we're not in that state yet, put a line end in leading_break and set "leading_blanks" to true
            clear(whitespaces)                        //    clearing any stuff from whitespaces
            parser->read_line(1)->leading_break
            leadings_blanks = true

    if leading_blanks:
        if leading_break[0]->'\n':
            if trailing_breaks.is_empty():
                string += ' '
            else
                string += trailing_breaks
                clear(trailing_breaks)

            clear(leading_break)
        else
            string += leading_break
            string += trailing_breaks
            clear(leading_break)
            clear(trailing_breaks)
    else
        string += whitespaces
        clear(whitespaces)

return string
```

### Paths

1. leading_blanks is false the entire loop

    This looks like the path that is taken when eating spaces between words, but NOT between a word and an EOL

2. leading_blanks is BEING SET TRUE this loop

    This paths looks the start of handling a line break. We need to encounter a YAML break (not blank), in which case
    we _clear any whitespaces since the last word was read_, set leading_break to a '\n' and set leading_blanks to true.

    Next we check if leading_blanks is true (it is) and leading_break has a '\n' (it does) and trailing_breaks is empty
    (it is) and we append a single space to the output string.


### Flow style line join rules (double quoted)

```
IF double_quoted AND $line ENDS WITH ['\\', isBreak!]
        KEEP ALL trailing whitespace
    AND
        REMOVE ALL leading whitespace on $line+1
ELSE
        REMOVE ALL trailing whitespace
    AND
        REMOVE ALL leading whitespace on $line+1

AND THEN

IF $line+1 IS ONLY WHITESPACE
        ADD ONE '\n'
ELSE
        ADD ONE ' '
```

<!-- markdownlint-disable-file -->

