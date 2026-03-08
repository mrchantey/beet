- multiline blockquotes are still a bit broken in ansi_term and markdown renderers:

```
// In
> *I tried eating one once but it didn't taste very nice*
>
> — Some fool
// expected out
> *I tried eating one once but it didn't taste very nice*
>
> — Some fool
// current - blank line in between
> *I tried eating one once but it didn't taste very nice*

> — Some fool

```
