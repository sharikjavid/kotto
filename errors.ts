export class RuntimeError extends Error {
    constructor(message: string) {
        super(message)
        this.name = "RuntimeError"
    }
}

export class Interrupt extends Error {
    value: any

    constructor(value: any) {
        super("LLM execution interrupted")
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

export class Exit extends Error {
    value: any

    constructor(value: any) {
        super("LLM execution exited")
        this.name = "Return"
        this.value = value
    }
}
