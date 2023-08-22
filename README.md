<h1 align="center">
  :robot: kotto
</h1>

<br/>

<p align="center">
<b>An agent framework that lets LLMs know how to use your code.</b>
</p>

<br/>

<p align="center">
  <a href="examples/hello.ts"><img src="https://kotto.land/static/hello-im-js.png" width="700"/></a>
</p>

<br/>

> [!WARNING]
> kotto is a very early stage project. Expect features and APIs to break frequently.

<br/>

- [Getting Started](#runner-getting-started)
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
        - [Classes](#classes)
        - [Exports](#exports)
    - [Event loop](#event-loop)
        - [`Exit`](#exit)
        - [`Interrupt`](#interrupt)
        - [`Feedback`](#feedback)
        - [Any other exception](#any-other-exception)
- [FAQ](#faq)
- [Contributing](#contributing)

## :runner: Getting Started

### Installation

Install the kotto CLI

```bash
curl -fsSL https://kotto.land/install.sh | sh
```

set your OpenAI API key

```bash
kotto config openai.key MY_SECRET_KEY
```

and run your first example

```bash
kotto run https://kotto.land/examples/hello.ts
```

### Hello, I'm a JavaScript runtime.

Create a file `hello.ts` and lay down the skeleton of a class:

```typescript
import * as kotto from "https://kotto.land/mod.ts"

class Hello {
    // Call this function with an introduction of yourself
    @kotto.use
    hello(message: string) {
        console.log(message)
        throw new kotto.Exit()
    }
}

export default () => new Hello()
```

Note the `@kotto.use` decorator and the comment: this is the key to exposing the hello method to the LLM backend and
explaining what we want to happen.

Now run the agent:

```bash
$ kotto run hello.ts
Hello, I am a JavaScript runtime.
```

Under the hood, kotto has statically generated a prompt set that includes the type signature of the `hello`
function as well as the comment above it. The model then predicts that it needs to call
the function with the argument `"Hello, I'm a JavaScript program."`.

We can get a bit more insight into what's going on by tuning up the log level:

```bash
kotto run --trace hello.ts
```

This will display a trace log of actions taken by the LLM. Like many other tools, kotto asks the LLM backend to
justify the reasoning behind an action choice - and that reasoning is displayed in dimmed text above each action.

### Type is context

Because the LLM knows the type signature of the `hello` function, we can use the type system to our advantage. Let's
change the example a bit:

```typescript
import * as kotto from "https://kotto.land/mod.ts"

class Hello {
    // Call this function with how you feel today
    @kotto.use
    hello(message: "happy" | "neutral" | "sad") {
        console.log(`I am feeling ${message} today.`)
        throw new kotto.Exit()
    }
}

export default () => new Hello()
```

and run it again:

```bash
$ kotto run hello.ts
I am feeling happy today.
```

We can also use custom types to document even more context:

```typescript
import * as kotto from "https://kotto.land/mod.ts"

type Feeling = {
    // How do you feel?
    state: "happy" | "neutral" | "sad"

    // Why do you feel this way?
    reason: string
}

class Hello {
    // Call this function saying you're happy to learn about kotto.
    @kotto.use
    hello({state, reason}: Feeling) {
        console.log(`I am feeling ${state} today, because ${reason}`)
        throw new kotto.Exit()
    }
}

export default () => new Hello()
```

kotto automatically adds type declarations (here, the `Feeling` type) to the internal prompt set.

```bash
$ kotto run hello.ts
I am feeling happy today, because I am excited to learn about kotto!
```

## :rocket: Examples

### Data from text

Since the type system is used to generate prompts, you can use the underlying LLM to extract data from real-world noisy
text data.

For example, [extract.ts](./examples/extract.ts) takes a string argument and extracts some info from it:

```bash
kotto run https://kotto.land/examples/extract.ts -- "I am 25 years old and I live in Paris"
```

### Chatbots

You can use kotto to build interactive chatbots that can leverage Deno's ecosystem of libraries to pack awesome
functionality into your agents.

To get you started, take a look at [chat.ts](./examples/chat.ts):

```bash
kotto run https://kotto.land/examples/chat.ts
```

### Automate stuff

You can use kotto to script agents that automate things for you.

For example, [fix.ts](./examples/fix.ts) is a small utility that will take a command and help you with getting what
you want with it:

```bash
kotto run https://kotto.land/examples/fix.ts -- egrep -e "???" MyFile.txt 
```

Another example is [summarise.ts](./examples/summarise.ts), which will take a GitHub repository, pull its README.md
and summarise it with the info you want:

```bash
kotto run https://kotto.land/examples/summarise.ts -- brokad/kotto
```

## :books: Documentation

### Building agents

#### Imports

Thanks to Deno's module loader, which supports importing from URLs, you only need one import to get started. Add this to
your agent modules:

```typescript
import * as kotto from "https://kotto.land/mod.ts"
```

This tracks the latest release. If you need a specific version, use:

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
between your agent and the LLM backend. In other words: your class goes on autopilot.

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

This exception will be unwound and the inner `Error` will be rethrown.

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
bounce back information to the LLM, which is especially useful on validating inputs it gives you:

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

If a method called by the LLM backend throws an exception and that exception is not one of the exceptions above, then
that exception will be repackaged as a system message and sent back to the LLM.

## FAQ

### How safe is it?

kotto lets an LLM decide what action to take next. And since LLMs are large and complicated models, it is difficult
to guarantee agents are safe against adversarial user inputs.

At the level of kotto, there are a few implemented backstops that can help.

One of them is that we **never** execute code coming directly from the LLM backend. We have a pure JSON-only interface
with the LLM,
asking it for data and returning it data. So the model is unable to have side effects that you didn't expose through the
content of your own code.

Another backstop is that *only* methods that are explicitly tagged with the [@use](#building-agents) annotation are
exposed to the
LLM. Therefore, only those methods are known to the model. Even the method's body is hidden from the model! So it only
knows of the public interface: the method name, its documentation, the method's arguments, the type declarations of
those arguments, etc. Basically it knows what you would otherwise know reading through a documentation page.

That being said, if security is a concern, you should always validate untrusted inputs and carefully consider the side
effects your agent can produce.

TL;DR: If you're dealing with untrusted user input, apply the same caution as you would when implementing any
public-facing API.

## Contributing

kotto is 100% a community effort to make LLM chains easy to build and use. And I'm so grateful you're willing to
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
