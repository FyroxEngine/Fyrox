# Contributors guidelines

This document explains how to contribute to the engine. Keep in mind, that this not a "must-obey" list; use common sense
when contributing something to the engine.

## How to contribute

- **Code** - writing code is the most obvious way of contribution. You can add missing features, fix bugs, improve
  existing tools, etc. See the [code section](#contributing-code) for more info.
- **Documentation** - documentation is the next place where you can contribute, this engine is already quite big and
  there's a bunch of undocumented code. If you're familiar with some undocumented API, don't hesitate - write
  documentation for it. You will save a lot of time for the next people who will be using this API and they will be
  grateful that the docs exists.
  See the [documentation section](#contributing-documentation) for more info.
- **Make games** - the best way to understand what's missing or needs to be improved is by making games using the
  engine. When you find something missing or can be improved, don't hesitate - create an issue in this repository and
  may be somebody will spot this issue and fix it. Filing issues is always good, it clearly shows that there's something
  wrong and people can track the progress.
- **Report bugs** - if something does not work as it should, file an issue about it so the problem will be clear. Any
  software has edge cases, that could be hidden for a long time, until you need to do something non-trivial.
- **Donation** - if you don't have any time to write the code or docs, and you want to see the project alive, you should
  consider donating any amount of money to the developers of the engine.
- **Promote the engine** - write posts, make videos, share news about the engine on social media and so on.

## Contributing code

Common rules for code contributions:

- **Keep code clean** - name your variables and functions meaningfully. Try to not create god-like functions, that
  handles everything at once. Always compile your code before making a pull request.
- **Write documentation** - document your code. It should explain what the code does at high level. Do not include low
  level details in your documentation (unless you need to explain something, that is very important).
- **Format your code** - use `rustfmt` to format the code you're writing.
- **Write unit tests** - if you're adding new functionality to the engine, make sure to write unit tests for it. It is
  not always possible to write meaningful unit tests, for example, graphics can hardly be tested this way. In this case
  make sure to thoroughly test your code manually.
- **Describe your code** - it is important to explain why you wrote the code and what it does. Do not create pull
  requests with description like: "fixed bug", "added stuff", etc. It does not help anyone, instead write a proper
  description.
- **License** - include the content of `LICENSE.md` file at the top of any new source code file. Every line must be
  start from `\\ ` (two slashes with a space after them). You can add your own copyright line with dates, but you must
  keep the license unchanged (MIT).

When you're writing something for the editor, you can run its standalone version using `fyroxed` package like so
`cargo run --packaged fyroxed`. This way the editor will run without any plugins, and you can test your changes quickly
without a need to create a project and test there.

## Contributing documentation

Common rules for documentation contributions:

- **Write everything in English** - official API documentation and [the book](https://fyrox-book.github.io/) written in
  English. If you want to create a translation for the book, you should create your own repository.
- **Add code examples** - code snippets helps other developers to quickly understand how to use a function/method.
- **Use spell checker** - keep the docs clean and readable.
- **Expertise** - make sure that you understand the thing you're writing the docs for. Shallow docs are usually
  misleading, and sometimes they're even worse, than no documentation at all. 