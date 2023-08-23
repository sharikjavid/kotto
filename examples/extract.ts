import * as kotto from "../mod.ts";

type Data = {
  first_name?: string;

  age?: number;

  location?: string;

  sentiment?: "positive" | "negative";

  url?: string;
};

class Extract {
  constructor(public raw: string) {}

  /**
   * Obtain a raw string
   *
   * @returns {string} A raw string given by the user
   */
  @kotto.use
  getRawString(): string {
    return this.raw;
  }

  /**
   * End the task by extracting structured data from the raw string
   *
   * @param {Data} structured a structured Data object
   */
  @kotto.use
  setData(structured: Data) {
    console.log(structured);
    throw new kotto.Exit(structured);
  }
}

export default ({ argv }: kotto.AgentOptions) => {
  if (argv[0] === undefined) {
    console.error(`you must call this with a string to extract data from

to extract structured data from a string:

    kotto run https://kotto.land/examples/extract.ts -- "My name is Brody and I'm 21 years old"`);
    Deno.exit(1);
  }
  return new Extract(argv[0]);
};
