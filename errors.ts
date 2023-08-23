type RuntimeErrorParams = {
  code?: number;
  context?: Error;
};

export class RuntimeError extends Error {
  context?: Error;
  code?: number;

  constructor(message: string, { code, context }: RuntimeErrorParams = {}) {
    super(message);
    this.name = "RuntimeError";
    this.code = code;
    this.context = context;
  }
}

export class Interrupt extends Error {
  value: any;

  constructor(value: any) {
    super("LLM execution interrupted");
    this.name = "Interrupt";
    this.value = value;
  }
}

export class Feedback extends Error {
  constructor(message: string) {
    super(message);
    this.name = "Feedback";
  }
}

export class Exit extends Error {
  value: any;

  constructor(value?: any) {
    super("LLM execution exited");
    this.name = "Return";
    this.value = value;
  }
}
