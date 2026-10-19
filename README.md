<div align="center">
  <img src="resources/icons/hicolor/scalable/apps/icon.svg" width="180" alt="Writing Tools icon">
  <h2>Writing Tools</h2>
  <h5>Private, local AI-assisted writing tools for the COSMIC™ desktop.</h5>
  <img src="resources/screenshots/window-light.png#gh-light-mode-only">
  <img src="resources/screenshots/window-dark.png#gh-dark-mode-only">
</div>

## Features
Writing Tools offers Apple-style writing actions, rewrite, proofread, summarize, and tone adjustments, powered by a model running on your own machine. Your text is never sent to a cloud service.

- **Rewrite** text while keeping its meaning.
- **Proofread** for spelling, grammar, and punctuation.
- **Summarize** long passages into something short.
- **Adjust the tone** of your writing to fit the occasion.
- **Fully local.** Works with [Ollama](https://ollama.com/) and [LM Studio](https://lmstudio.ai/), so nothing leaves your computer.
- Built with [libcosmic](https://github.com/pop-os/libcosmic) to match the rest of the COSMIC desktop.

## Local model setup

Writing Tools talks to a model server running on your machine. Select the provider under **Local Model**, then set its endpoint and the identifier of the loaded model.

### Ollama

Install [Ollama](https://ollama.com/), then download and start a model:

```sh
ollama run llama3.2
```

The default endpoint is `http://127.0.0.1:11434` and the default model is `llama3.2`.

### LM Studio

Load a model in [LM Studio](https://lmstudio.ai/) and start its local server. Then, in Writing Tools:

1. Select **LM Studio** as the provider.
2. Set the endpoint to the server address, normally `http://127.0.0.1:1234`.
3. Enter the identifier of the loaded model.

Writing Tools sends native LM Studio requests to `/api/v1/chat`.

## Installation

A [justfile](./justfile) is included for the [casey/just](https://github.com/casey/just) command runner.

```sh
git clone https://github.com/cosmic-utils/cosmic-ext-writing-tools.git
cd cosmic-ext-writing-tools
sudo just install
```

| Command           | Description                                                  |
| ----------------- | ------------------------------------------------------------ |
| `just`            | Builds the application (default `just build-release` recipe) |
| `just run`        | Builds and runs the application                              |
| `just install`    | Installs the project into the system                         |
| `just vendor`     | Creates a vendored tarball                                   |
| `just build-vendored` | Compiles with vendored dependencies from that tarball    |
| `just check`      | Runs clippy to check for linter warnings                     |
| `just check-json` | Same as `check`, for IDEs that support LSP                   |

## Build

```sh
git clone https://github.com/cosmic-utils/cosmic-ext-writing-tools.git
cd cosmic-ext-writing-tools
cargo build
```

### Dependencies

- [libcosmic](https://github.com/pop-os/libcosmic?tab=readme-ov-file#building)
- [Ollama](https://ollama.com/) or [LM Studio](https://lmstudio.ai/) at runtime

## Packaging

If packaging for a Linux distribution, vendor dependencies locally with the `vendor` rule, and build with the vendored sources using the `build-vendored` rule. When installing files, use the `rootdir` and `prefix` variables to change installation paths.

```sh
just vendor
just build-vendored
just rootdir=debian/cosmic_ext_writing_tools prefix=/usr install
```

It is recommended to build a source tarball with the vendored dependencies, which can typically be done by running `just vendor` on the host system before it enters the build environment.

## Translators

[Fluent](https://projectfluent.org/) is used for localization of the software. Fluent's translation files are found in the [i18n directory](./i18n). To add a new translation, copy the [English (en) localization](./i18n/en), rename `en` to the desired [ISO 639-1 language code](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes), and provide a translation for each [message identifier](https://projectfluent.org/fluent/guide/hello.html). If no translation is necessary, the message may be omitted.

## Copyright and licensing

Copyright 2026 © Eduardo Flores

Writing Tools is released under the terms of the [GPL-3.0](./LICENSE).
