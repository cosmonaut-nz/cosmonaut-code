# >_ cosmonaut code

<img src="assets/img/cosmonaut_logo_trans.png" width="12%" height="12%">
>_ we are cosmonaut

## purpose

to be honest, we built this tool because we needed it for the work we do. sure there are some great tools out there, but none of them quite hit the mark for our needs.

it's in pure rust! :p (so far...)

### goals

1. provide a viable local source tree and project analysis tool using various tools and generative ai agents. ideal for pre-pr checking by a developer.
2. create a tool that can be added to a build to ensure that no significant errors are in the code.
3. allow code owners to check the overall health of the code in a simple way.
4. output an actionable report that will improve the code base.

### use cases

1. new developers on a project.
2. new maintainers of a codebase or take over of code base / project.
3. entry-point for an external audit of code base.
4. entry-point for a security audit of code base.
5. code owner reporting on technical-debt and general health of asset.

## how to use

### installation

`git clone https://github.com/cosmonaut-nz/cosmonaut-code.git`

`cd costmonaut-code`

`cargo run`

## contributing

yes please!!

see [contributing](CONTRIBUTING.md) for the rules, they are standard though.

## work status

we do our best to release working code.

status today is: *"it works, but it is not pretty or very user friendly."*

[X] load local repository

[ ] load github repository

[ ] load gitlab code repository

[X] enable openai review of code

[ ] enable google palm review of code

[ ] enable anthropic claud review of code

[ ] enable meta llama review of code

[ ] enable local llm review of code

[ ] comparison of different llms revie output on same code (this could be very cool!)

[X] output in json

[ ] output in pdf

[ ] run from pipeline (github actions, gitlab-ci)

[ ] user interface (maybe not in rust)

## code structure

The code is broken into modules to ensure a separation of concerns:

- `provider` - managing the LLM providers and API calls to review code files
- `review` - managing the review of the repository, including handling the filesystem and reading in of files
- `settings` - a set of data structures that enable easy application configuration

```plaintext
src
├── lib.rs
├── main.rs
├── provider
│   ├── api.rs
│   ├── mod.rs
│   └── prompts.rs
├── review
│   ├── data.rs
│   ├── mod.rs
│   └── tools.rs
└── settings
    └── mod.rs
```
