export class RuntimeError extends Error {
    constructor(message: string) {
        super(message)
        this.name = "RuntimeError"
    }
}

export class Interrupt extends Error {
    constructor(value: any) {
        super("LLM execution interrupted by user")
        this.name = "Interrupt"
        this.value = value
    }
}

export class Feedback extends Error {
    constructor(message: string) {
        super(message)
        this.name = "Feedback"
    }
}

