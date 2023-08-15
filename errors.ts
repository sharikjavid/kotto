export class RuntimeError extends Error {
    constructor(message: string) {
        super(message)
        this.name = "RuntimeError"
    }
}

export class Interrupt extends Error {
    constructor(message: string) {
        super(message)
        this.name = "Interrupt"
    }
}
