<h1 align="center">
  kotto
</h1>

<p align="center">
  <i align="center">An agent framework that tells LLMs how to use your code :robot:</i>
</p>

<br/>

<p align="center">
  <a href="https://snappify.com/view/7439fa7d-84d0-4284-b641-739242eb7ea1?autoplay=1">
    <img src="https://kotto.land/static/type-is-context-round.png" width="100%"/>
  </a>
</p>

<p align="center">
  <a href="https://snappify.com/view/7439fa7d-84d0-4284-b641-739242eb7ea1?autoplay=1">
    Check out the animated demo.
  </a>
</p>

## Introduction

`kotto` is an agent framework that statically builds context-efficient prompts from plain JavaScript/TypeScript, exporting your code so it can be consumed at runtime by popular large 
language models (LLMs).

This allows you to develop any app that uses LLMs without ever doing manual prompting and, instead, leverage the type system and JSDoc strings as context to inform model predictions. And go from zero to agent in under a minute (check out the [Hello, World!](#hello-im-a-javascript-runtime))!

### Why Kotto?
- 🤖 No manual prompting
- 🚀 Dead simple to use
- ⚡️ Great for serverless environments like [Deno Deploy](https://deno.com/deploy)
- 🚶‍ Step-by-step debugging, with introspection
- 🦕 Built for Deno, with Rust 🦀

### Progress

- [x] Prompts from class methods.
- [x] Trace logging and debug mode.
- [x] Customize generated prompts.
- [x] Self-exit.
- [x] Send system messages for feedback.
- [ ] Prompts from class attributes/fields.
- [ ] Builtin types (e.g. Request/Response).
- [ ] Builtin functions (e.g. fetch, Deno namespace).
- [ ] Reinject context for extended sessions.
- [ ] Support more LLM providers.

<br/>

> [!WARNING]
> kotto is a very early stage project. Expect features and APIs to break frequently.

<br/>

- [Getting Started](#runner-getting-started)
    - [Requirements](#requirements)
    - [Installation](#installation)
    - [Hello, world!](#hello-im-a-javascript-runtime)
    - [Type is context](#type-is-context)
- [Examples](#rocket-examples)
    - [Data from text](#data-from-text)
    - [Chatbots](#chatbots)
    - [Automate stuff](#automate-stuff)
- [Documentation](#books-documentation)
    - [Building agents](#building-agents)
        - [Imports](#imports)
        - [Classes](#agents)
        - [Exports](#exports)
    - [Event loop](#event-loop)
        - [`Exit`](#exit)
        - [`Interrupt`](#interrupt)
        - [`Feedback`](#feedback)
        - [Any other exception](#any-other-exception)
- [FAQ](#faq)
- [Contributing](#contributing)

## :runner: Getting Started

### Requirements

kotto is built on top of [Deno](https://deno.land/), a secure runtime for JavaScript and TypeScript. You'll need to
install it to run kotto agents. Use the [official guide](https://deno.land/manual/getting_started/installation) to get started.

kotto also uses [OpenAI's API](https://platform.openai.com/docs/introduction) as the only supported LLM backend is 
gpt-3.5 (more coming soon!). So you'll need an OpenAI API key. You can generate one [over here](https://platform.openai.com/account/api-keys).

### Installation

Install the kotto CLI

```bash
deno install -A -n kotto https://kotto.land/cli.ts
```

and install [kottoc](./kottoc) (a Rust companion that generates prompts):

```bash
kotto upgrade
```

Set your OpenAI API key

```bash
kotto config openai.key MY_SECRET_KEY
```

and run your first example

```bash
kotto run https://kotto.land/examples/hello.ts
```

### Hello, I'm a JavaScript runtime.

Let's our own agent from scratch. Create a file `hello.ts` and lay down the skeleton of a class:

```typescript
import { use } from "https://kotto.land/mod.ts"

class Hello {
    @use
    hello(message: string) {
        console.log(message)
    }
}

export default () => new Hello()
```

Note the `@use` decorator: this is the key to exposing the `hello` method to the LLM backend.

Now run the agent:

```bash
$ kotto run hello.ts
Hello, World!
```

Under the hood, kotto has statically generated a prompt set that includes the type signature of the `hello`. The model 
then predicts that it needs to call the function with the argument `"Hello, World!"`. And that message gets written to 
stdout because of the `console.log` line.

We can also use comments to tell the model a bit more about what we want. Let's ask it to speak [High Valyrian](https://gameofthrones.fandom.com/wiki/High_Valyrian):

```typescript
import { use } from "https://kotto.land/mod.ts"

class Hello {
    @use
    // This function should be called with a message in High Valyrian
    hello(message: string) {
        console.log(message)
    }
}

export default () => new Hello()
```

and run it again:

```bash
$ kotto run hello.ts
Valar Morghulis!
```

We can get a bit more insight into what's going on by running the agent in debug mode:

```text
$ kotto debug hello.ts
trace: adding 'hello' to scope 
trace:     ╭ Since the program states that the function 'hello' should be called with 
             a message in High Valyrian, I will call this function to pass the appropriate 
             message to it.
trace:  call hello("Valar morghulis")
Valar morghulis
trace:     ⮑  returns null
trace:     ╭ After executing the 'hello' function, the code does not have any more 
             instructions to execute. Therefore, I should call the 'builtins.exit' function
             to gracefully terminate the program.
trace:  exit null
```

This will display a trace log of actions taken by the LLM along with (in dimmed text) the model's explanation for the
choice.

Note that, by default, the model is given a `builtins.exit` function to call when it predicts it is done. This behaviour
can be overridden by giving `kotto run` the `--no-exit` flag. You will then have to terminate the agent by throwing 
an [`Exit` exception](#exit) (the next example shows how to do that).

### Type is context

Because the LLM knows the type signature of the `hello` function, we can use the type system to our advantage. Let's
change the example a bit:

```typescript
import { use, Exit } from "https://kotto.land/mod.ts"

class Hello {
    @use
    // Call this function with how you feel today
    hello(message: "happy" | "neutral" | "sad") {
        console.log(`I am feeling ${message} today.`)
        throw new Exit()
    }
}

export default () => new Hello()
```

Because `message` now has a union type, it will be called only with one of the three stated options: "happy", 
"neutral" or "sad". Let's run it again:

```bash
$ kotto run hello.ts
I am feeling happy today.
```

With no additional context, the model has no choice but to hallucinate a feeling - which tends to be happy. Also note 
the `throw new Exit()` line, which terminates the agent after the first call to `hello`.

We can also use custom/nested types to document even more context:

```typescript
import { use, Exit } from "https://kotto.land/mod.ts"

type Feeling = {
    // How do you feel?
    state: "happy" | "neutral" | "sad"

    // Why do you feel this way?
    reason: string
}

class Hello {
    @use
    // Call this function saying you're happy to learn about kotto.
    hello({state, reason}: Feeling) {
        console.log(`I am feeling ${state} today, because ${reason}`)
        throw new Exit()
    }
}

export default () => new Hello()
```

Kotto automatically adds type declarations (here, the `Feeling` type) and their documentation comments to the internal 
prompt set.

```bash
$ kotto run hello.ts
I am feeling happy today, because I am excited to learn about kotto!
```

## :rocket: Examples

### Data from text

Kotto generates LLM prompts from your code's type signatures and comments. This means you can use type declarations to
define what you want from the LLM.

For example, [extract.ts](./examples/extract.ts) takes a string argument and extracts the following type:

```typescript
type Data = {
    first_name?: string,

    age?: number,

    location?: string,

    sentiment?: "positive" | "negative"
}
```

Let's run it:

```bash
$ kotto run  https://kotto.land/examples/extract.ts -- \
  "I'm Marc, I am 25 years old, I live in Paris and I'm very happy"
{
  first_name: "Marc",
  age: 25,
  location: "Paris",
  sentiment: "positive"
}
```

### Chatbots

You can also use kotto to build interactive chatbots. Deno has a large ecosystem of modules that you can use to pack
awesome functionality into your agents. Then you can deploy them on [Deno Deploy](https://deno.com/deploy).

To get you started, take a look at [chat.ts](./examples/chat.ts):

```bash
kotto run https://kotto.land/examples/chat.ts
```

### Automate stuff

You can also use kotto to script agents that automate things for you in a clever way. If you've ever found yourself 
constantly copy/pasting things into a ChatGPT prompt, you'll love this.

For example, [fix.ts](./examples/fix.ts) is a small utility that will take a command and help you with getting what
you want with it:

```text
$ kotto run https://kotto.land/examples/fix.ts -- egrep /var/log/sshd.log 
[...] I want to match all IPv4 addresses in the file
$ egrep -e \b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b /var/log/sshd.log
```

Another example is [summarise.ts](./examples/summarise.ts), which will take a GitHub repository, pull its README.md
and summarise it with the info you want.

## :books: Documentation

### Building agents

#### Imports

All you need is one import to get started building your own agents with kotto:

```typescript
import * as kotto from "https://kotto.land/mod.ts"
```

This tracks the latest release (recommended). If you ever need to pin a specific version, use:

```typescript
import * as kotto from "https://kotto.land/@0.1.0/mod.ts"
```

#### Agents

Any class can become an agent. Just make sure you decorate at least one of its methods with `@use`:

```typescript
import { use } from "https://kotto.land/mod.ts"

class MyAgent {
    @use
    myMethod() {
        // ...
    }
}
```

> [!IMPORTANT]
> The LLM backend does not know of any other method than the ones you decorate with `@use`.

When a method is decorated with `@use`, its type signature and its JSDoc/comments (if there are any) are added to the
prompt set. However, the method's body is kept hidden.

#### Exports

Agent modules must have a default export that is a callable and returns an instance of your agent.

```typescript
export default () => new MyAgent()
```

This function can accept an argument of type `AgentOptions`:

```typescript
export default ({ argv }: AgentOptions) => {
    // ...do something with argv
    return new MyAgent()
}
```

The `AgentOptions` type is defined as:

```typescript
type AgentOptions = {
    // The arguments passed to the agent on the command line (all the arguments after '--')
    argv: string[]
}
```

### Event loop

When you run an agent with `kotto run`, the runtime will enter an event loop. It will keep bouncing back and forth
between your code and the LLM backend.

There are exceptions you can throw to control that event loop:

#### `Exit`

This exception will be unwound, stop the event loop and exit the runtime.

```typescript
import { use, Exit } from "https://kotto.land/mod.ts"

class MyAgent {
    @use
    myMethod() {
        // Exit the event loop, and the runtime
        throw new Exit()
    }
}
```

#### `Interrupt`

This exception will be unwound and the inner `Error` will be rethrown by the event loop handler.

```typescript
import { use, Interrupt } from "https://kotto.land/mod.ts"

class MyAgent {
    @use
    async readFile(path: string) {
        try {
            return await Deno.readTextFile(path)
        } catch (e) {
            // Exit the event loop, rethrowing the error
            throw new Interrupt(e)
        }
    }
}
```

#### `Feedback`

This exception will be unwound and repackaged as
a [system message](https://platform.openai.com/docs/api-reference/chat/create) to the LLM backend. You can use it to
bounce back information to the LLM:

```typescript
import { use, Feedback } from "https://kotto.land/mod.ts"

class MyAgent {
    @use
    howOldAreYou(age: number) {
        // Send a system message to the LLM backend
        if (age < 0) throw new Feedback("age cannot be negative")
    }
}
```

#### Any other exception

Any other exception thrown by your code (that is not caught before reaching a @use method) will be unwound and repackaged
as a system message to the LLM backend. This will give it a chance to recover from the error and continue its course.

## FAQ

### Does kotto let LLMs run arbitrary code?

Hell no! There is only a single JSON-in/JSON-out interface with the LLM backend, so we never execute code coming from it.

### Why do I need an OpenAI key?

The only LLM backend supported by kotto is gpt-3.5 (but more are coming soon!).

### Is my code sent to OpenAI?

Some of it. Kotto generates prompts from your code's type signatures and comments. It then sends those prompts to the
LLM backend. The LLM backend then sends back a completion, which kotto uses to run its event loop.

The body of methods is never part of that prompt set because that tends to pollute the context window and confuse the 
model. So that code is never sent to OpenAI. And methods that are **not** decorated with [`@use`](#agents) are completely
omitted so they remain private.

On the other hand, method names, argument names and type declarations are indeed sent to OpenAI - but only for 
methods tagged with `@use`.

### Is any data sent to `kotto.land`?

No! We use the `kotto.land` domain as an easy import path (which works thanks to Deno's awesome 
[module loader](https://deno.land/manual@v1.36.2/basics/modules#remote-import)). But kotto works 100% locally as an 
orchestrator between your code and the LLM backend.

## Contributing

Kotto is 100% a community effort to make LLM chains easy to build and use. And I'm so grateful you're willing to
help!

If you have found a bug or have a suggestion for a feature you'd like, open
an [issue](https://github.com/brokad/kotto/issues/new). PRs are of course always
welcome!

If you have a question to ask or feedback to give, be it good or bad, please start
a [discussion](https://github.com/brokad/kotto/discussions/new?category=ideas).

If you feel like helping with the implementation, get in touch!

[LangChain]: https://python.langchain.com/docs/get_started/introduction.html

[LlamaIndex]: https://gpt-index.readthedocs.io/en/latest/

[marvin]: https://github.com/PrefectHQ/marvin
