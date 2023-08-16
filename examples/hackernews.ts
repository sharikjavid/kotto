import ai, { Agent } from "../mod.ts"

const HN_URL = "https://hacker-news.firebaseio.com"

type Id = number

type Item = {
    // The id of this item
    id: Id,

    // An array of children items
    kids: Id[]

    // Items can have one of two types: "story" or "comment"
    // Stories are equivalent to "posts". Comments are items
    // attached to a parent story.
    type: "story" | "comment"

    // a link attached to this post
    url?: string

    // the text content of the post
    text?: string
}

type Story = {
    story: Item,
    comments: Item[]
}

async function fetchHN<T>(rel: string): Promise<T> {
    const resp = await fetch(`${HN_URL}${rel}`)
    return await resp.json()
}

function fetchItem(id: Id): Promise<Item> {
    return fetchHN(`/v0/item/${id}.json?print=pretty`)
}

class LabelHN {
    keywords: Set<string> = new Set()

    queue: Id[] = []

    /**
     * Retrieve the list of existing keywords.
     *
     * Existing keywords should be prioritised ahead of new (invented) keywords.
     *
     * @returns The list of existing keywords
     */
    @ai.use
    getExistingKeywords(): string[] {
        return Array.from(this.keywords)
    }

    /**
     * Get the next story to process.
     *
     * @returns {Story} A story item and its first three comments (if they exist).
     */
    @ai.use
    async nextStory(): Promise<Story> {
        const story = await fetchItem(this.queue.pop()!)

        const comments = []
        if (story.kids !== undefined) {
            for (const id of story.kids.slice(0, 3)) {
                comments.push(await fetchItem(id))
            }
        }

        return { story, comments }
    }

    /**
     * Associate a set of keywords with an item of type "story".
     *
     * This will record `keywords` as being a match for the item with id `id`. Keep the keywords
     * to general concepts only.
     *
     * @param {Id} id The id of the story item
     * @param {string[]} keywords A set of keywords to associate with the item.
     */ 
    @ai.use
    setMatchingKeywords(id: Id, keywords: string[]) {
        // If the model is proposing too many new keywords, force it to summarise...
        const new_keywords = keywords.filter(item => !this.keywords.has(item))
        if (new_keywords.length > 1)
            throw new ai.Feedback(`'${new_keywords}' are too many new keywords (there can only be one new keyword)! Keep it general.`)

        // ...otherwise save the keywords and end this task
        new_keywords.forEach(item => this.keywords.add(item))

        throw new ai.Exit({
            post_id: id,
            keywords: keywords
        })
    }
}

const myCrawler = new LabelHN()

// Queue up the 10 top stories on HN right now
myCrawler.queue = await fetchHN<Id[]>("/v0/topstories.json?print=pretty")
    .then((stories) => stories.slice(0, 10))

// While there are posts in the queue...
while (myCrawler.queue.length != 0) {
    // ...run the agent on the next item, until it reaches an `ai.Exit` point
    await ai.run(myCrawler)
}
