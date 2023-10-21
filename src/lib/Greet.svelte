<script lang="ts">
    import {invoke} from "@tauri-apps/api/tauri";

    type File = {
        name: string;
        path: string;
        is_directory: boolean;
        icon: string;
        ext: string;
    };

    let results: Array<File> = [];
    let input = "";

    async function search() {
        const res = JSON.parse(await invoke("search", {input, path: "."}));
        results = res;
    }

    async function open_dir(dir: string) {
        console.log(dir);
        const res = JSON.parse(await invoke("open_dir", {dir}));
        results = res;
    }

    function handleKeyDown(event: KeyboardEvent) {
        if (event.key === 'Enter' || event.key === ' ') {
            // open_dir();
        }
    }
</script>

<div class="container mx-auto p-4">
    <form on:submit|preventDefault={search}>
        <input
                type="text"
                bind:value={input}
                placeholder="Search..."
                class="p-2 border rounded"
        />
        <button type="submit" class="p-2 ml-2 bg-blue-500 text-white rounded">Search</button>
    </form>

    <div class="mt-4">
        {#each results as file}
            <div tabindex="0" role="button" aria-pressed="false" on:click={() => open_dir(file.path)}
                 on:keydown={handleKeyDown} class="flex items-center p-2 border-b hover:bg-gray-400">
                <img src={file.icon} alt={file.name} class="w-6 h-6 mr-2"/>
                <div class="flex-1">
                    <div class="font-bold">{file.name}</div>
                    <div class="text-sm text-gray-500">{file.path}</div>
                </div>
                <!--<div class="text-sm text-gray-500 mr-2">{file.size}</div>-->
                <div class="text-sm text-gray-500">{file.ext != "dir" || !file.ext.includes('.', 0) ? '.' + file.ext : file.ext}</div>
            </div>
        {/each}
    </div>
</div>
