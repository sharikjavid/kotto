import ai from "../mod.ts"

type Repository = {
    full_name: string
    stargazers_count: number
}

class TopTrending extends ai.Agent {
    /**
     * Get the current datetime.
     *
     * @returns The current date and time.
     */
    @ai.use
    async getCurrentDate(): Date {
        return new Date()
    }
    
    /**
     * Given an url, method and headers, send an HTTP request and waits for the response, saving it
     * in the state of this object.
     *
     * Call this method with the parameters required to return the top 3 GitHub repositories that
     * were created in the past week, sorted by number of stars.
     *
     * @param {string} url The base URL to which to make the request (including query string parameters)
     * @param {string} method The HTTP method of the request to make
     * @param {Record<string, string>} headers A key-value record of headers of the request to make
     * @returns The first 3 items of the response body, decoded from JSON
     */
    @ai.use
    async getRepositories(url: string, method: string, headers: Record<string, string>): Repository[] {
        try {
            const resp = await fetch(url, {
                method,
                headers,
            })
            const data = await resp.json()
            return data.items.slice(0, 3).map((repo) => {
                return {
                    full_name: repo.full_name,
                    stargazers_count: repo.stargazers_count
                }
            })
        } catch (err) {
            throw new Error(`the request is incorrect, and could not be completed`)
        }
    }

    /**
     * Congratulate a top repository for its achievement.
     *
     * Call this successively for each top repository returned by `getRepositories`
     *
     * @param {Repository} repo A repository
     * @param {string} congratulation A message to send the owners of the repository, to
     *        congratulate them! Make it a joke about the repo's name.
     */
    @ai.use
    async congratulate(repo: Repository, congratulation: string) {
        console.log(`${repo.full_name} has ${repo.stargazers_count} stars! ${congratulation}`)
    }

    /**
     * End the task.
     *
     * Call this when all the top repositories have been congratulated.
     */
    @ai.use
    async done() {
        this.resolve()
    }
}

await ai.run(new TopTrending())
