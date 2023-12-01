# >_ cosmonaut code

<img src="assets/img/cosmonaut_logo_trans.png" width="12%" height="12%"> >_ we are cosmonaut

[![Rust Check](https://github.com/cosmonaut-nz/cosmonaut-code/actions/workflows/rust-check.yml/badge.svg)](https://github.com/cosmonaut-nz/cosmonaut-code/actions/workflows/rust-check.yml)

## purpose

it's a code explorer, explainer and assessment tool.

to be honest, we built this tool because we needed it for the work we do. sure there are some great tools out there, but none of them quite hit the mark for our needs.

it's in pure rust! :p (so far...)

### goals

1. provide a viable tool for local codebase analysis.
2. helps new developers to quickly get up to speed on a large or legacy codebase.
3. provide a tool that will help developers and code maintainers manage their code and start a conversation.
4. allow code owners to check the overall health of the code in a simple way.
5. output an actionable report that will improve the code base.

### non-goals

1. provide a tool that will automagically improve a codebase.
2. provide a tool that does not require further review or analysis of the code.
3. provide a tool that does not require thinking or discussion.

### use cases

1. new developers on a project.
2. new maintainers of a codebase or take over of code base / project.
3. entry-point for an external audit of code base.
4. entry-point for a security audit of code base.
5. code owner reporting on technical-debt and general health of asset.

## how to use

### disclaimer

this is really early days. running over a really big repo with the latest model will be super slow and likely fail. we've tested it up to ~1500 code files; worked 50% of the time, what with timeouts etc. cost about $5US. your mileage may vary. i think the value will come when it can be run over multiple models and compared and filtered.

right now it's a barebones offering. it kinda works, and we have got value from it, but there is a lot more to do. but it's been fun to do.

### installation

install rust:

`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

clone the repo:

`git clone https://github.com/cosmonaut-nz/cosmonaut-code.git`

`cd cosmonaut-code`

Add a `settings.json` with the following:

```json

{
    "sensitive": {
        "api_key": "[YOUR_OPENAI_API_KEY]",
        "org_id": "[YOUR_OPENAI_ORG_ID]",
        "org_name": "[YOUR_OPENAI_ORG_NAME]"
    },
    "repository_path": "[FULL_PATH_TO_REPO]",
    "report_output_path": "[FULL_PATH_TO_OUTPUT]"
}

```

```bash

export SENSITIVE_SETTINGS_PATH=[PATH_TO_YOUR_SETTINGS.JSON]

```

_Optional_

edit the `settings/default.json` to change the current defaults.

see [OpenAI Models](https://platform.openai.com/docs/models/gpt-4-and-gpt-4-turbo) for details

review type is:

1. full review
2. security review only
3. mock run, not using LLM for code review

output types are:

1. `html`
2. `json`

```json

{
    "providers": [
        {
            "name": "openai",
            "service": "gpt-4",
            "model": "gpt-4-1106-preview",
            "api_url": "https://api.openai.com/v1/chat/completions"
        }
    ],
    "default_provider": "openai",
    "output_type": "json",
    "review_type": 1
}

```

run:

`cargo run`

later there will be downloadable binaries so you don't have to install rust.

## contributing

yes please!!

see [contributing](CONTRIBUTING.md) for the rules, they are standard though.

## work status

we do our best to release working code. we hacked this out pretty quickly so the code's quality is not all that right now.

status today is: *"it works, but it is not pretty or very user friendly."*

## outline tasks

- [X] load local repository
- [X] enable openai review of code
- [X] output in json
- [X] output in html
- [ ] packaging so user can either install via `cargo install` or download the binary.
- [ ] output in pdf
- [ ] tune the prompts for clarity and accuracy
- [ ] GitHub Actions integration
- [ ] enable private llm review of code (likely llama-based)
- [ ] proper documentation
- [ ] GitLab pipeline integration
- [ ] enable google palm review of code
- [ ] enable anthropic claud review of code
- [ ] enable meta llama review of code
- [ ] comparison of different llms review output on same code (this could be very cool!)

## code

### structure

The code is broken into modules to ensure a separation of concerns:

- `provider` - managing the LLM providers and API calls to review code files
- `review` - managing the review of the repository, including handling the filesystem and reading in of files
- `settings` - a set of data structures that enable easy application configuration
- `common` - a set of common utility functions, macros and alike

```plaintext
.
├── build.rs
└── src
    ├── common
    │   └── mod.rs
    ├── main.rs
    ├── provider
    │   ├── api.rs
    │   ├── mod.rs
    │   └── prompts.rs
    ├── review
    │   ├── code.rs
    │   ├── data.rs
    │   ├── mod.rs
    │   ├── report.rs
    │   └── tools.rs
    └── settings
        └── mod.rs
```
