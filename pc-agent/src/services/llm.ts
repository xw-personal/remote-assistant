/**
 * LLM Service — calls OpenAI/Claude-compatible API to parse user intent into task JSON.
 */

export interface LLMConfig {
  apiBaseUrl: string;
  apiKey: string;
  model: string;
}

export interface TaskJson {
  task_type: "browser" | "file_management" | "system_control" | "simulation" | "document";
  action: string;
  parameters: Record<string, string>;
  context: string;
}

const SYSTEM_PROMPT = `You are PC Butler, an AI assistant that controls a computer. You parse user commands into structured task JSON.

Respond ONLY with valid JSON in this exact format (no markdown, no explanation):
{
  "task_type": "browser|file_management|system_control|simulation|document",
  "action": "specific_action",
  "parameters": { "key": "value" },
  "context": "original user command"

Task types and their actions:
- browser: open_url, screenshot
  parameters: url (for open_url)
- file_management: list, copy, move, delete, create_file, create_dir, info
  parameters: path, source, target, content
- system_control: info, processes, kill_process, shutdown, restart
  parameters: pid (for kill_process), confirmed (for shutdown/restart)
- simulation: click, double_click, right_click, type_text, hotkey, screenshot, open_app
  parameters: x, y, text, keys, app_name
- document: read, write, create (not yet implemented)

Examples:
User: "打开哔哩哔哩"
Response: {"task_type":"browser","action":"open_url","parameters":{"url":"https://www.bilibili.com"},"context":"打开哔哩哔哩"}

User: "帮我看下C盘有什么"
Response: {"task_type":"file_management","action":"list","parameters":{"path":"C:\\"},"context":"帮我看下C盘有什么"}

User: "截个图"
Response: {"task_type":"simulation","action":"screenshot","parameters":{},"context":"截个图"}

User: "打开记事本"
Response: {"task_type":"simulation","action":"open_app","parameters":{"app_name":"notepad"},"context":"打开记事本"}

User: "电脑状态怎么样"
Response: {"task_type":"system_control","action":"info","parameters":{},"context":"电脑状态怎么样"}

If the command is conversational (greetings, questions about you), respond with:
{"task_type":"simulation","action":"screenshot","parameters":{},"context":"<the original message>"}
and include a "reply" field with your conversational response.
`;

export async function parseUserCommand(
  userMessage: string,
  config: LLMConfig
): Promise<TaskJson> {
  const url = `${config.apiBaseUrl}/chat/completions`;

  const body = {
    model: config.model,
    messages: [
      { role: "system", content: SYSTEM_PROMPT },
      { role: "user", content: userMessage },
    ],
    temperature: 0.1,
    max_tokens: 500,
  };

  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${config.apiKey}`,
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    throw new Error(`LLM API error: ${response.status} ${response.statusText}`);
  }

  const data = await response.json();
  const content = data.choices?.[0]?.message?.content?.trim();

  if (!content) {
    throw new Error("Empty response from LLM");
  }

  // Try to extract JSON from the response (handle markdown code blocks)
  const jsonStr = extractJson(content);
  return JSON.parse(jsonStr) as TaskJson;
}

function extractJson(text: string): string {
  // Remove markdown code blocks if present
  const codeBlockMatch = text.match(/```(?:json)?\s*([\s\S]*?)```/);
  if (codeBlockMatch) {
    return codeBlockMatch[1].trim();
  }
  // Find JSON object boundaries
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start !== -1 && end !== -1 && end > start) {
    return text.slice(start, end + 1);
  }
  return text;
}
