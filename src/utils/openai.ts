import OpenAI from 'openai';

export async function generatePrDetails(diff: string, specContent: string): Promise<{ title: string; summary: string }> {
    const apiKey = process.env.OPENAI_API_KEY;
    if (!apiKey) {
        throw new Error('OPENAI_API_KEY is not set');
    }

    const openai = new OpenAI({ apiKey });

    const prompt = `
You are a helpful assistant that generates Pull Request titles and summaries.
Based on the following code diff and the original spec, please generate a concise PR title and a detailed summary.

Spec:
${specContent}

Diff:
${diff.substring(0, 10000)} // Truncate diff to avoid token limits if necessary

Output format (JSON):
{
  "title": "PR Title",
  "summary": "PR Summary in Markdown"
}
`;

    const response = await openai.chat.completions.create({
        model: 'gpt-4o',
        messages: [{ role: 'user', content: prompt }],
        response_format: { type: 'json_object' },
    });

    const content = response.choices[0].message.content;
    if (!content) {
        throw new Error('Failed to generate PR details');
    }

    return JSON.parse(content);
}
