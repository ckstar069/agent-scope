import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <div className="min-h-screen bg-gray-900 text-gray-100 flex flex-col items-center justify-center p-8">
      <h1 className="text-3xl font-bold mb-4">ptv</h1>
      <p className="text-gray-400 mb-8">Project Template Visualizer</p>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
        className="flex gap-2 mb-4"
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
          className="px-4 py-2 rounded bg-gray-800 border border-gray-700 text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500"
        />
        <button
          type="submit"
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded font-medium transition-colors"
        >
          Greet
        </button>
      </form>
      {greetMsg && (
        <p className="text-green-400 font-mono">{greetMsg}</p>
      )}
    </div>
  );
}

export default App;
