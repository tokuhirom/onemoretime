<script>

    import {onDestroy, onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/tauri";
    import {getKeyName} from "./keycode.js";
    import {flagsToString} from "./flags.js";

    let event_log = [];

    async function updateEventLog() {
        const r = await invoke("get_event_log", {});
        r.reverse();
        event_log = r;
    }

    let intervalId;

    onMount(async () => {
        await updateEventLog();

        // TODO fetching events by setInterval is not so good.
        // i want to push events from rust side, but it's bit hard.
        intervalId = setInterval(updateEventLog, 1000);
    });
    onDestroy(() => {
        clearInterval(intervalId);
    });

    function formatEpochToHHMMSS(epochSeconds) {
        let date = new Date(epochSeconds * 1000);
        let hours = date.getHours().toString().padStart(2, '0');
        let minutes = date.getMinutes().toString().padStart(2, '0');
        let seconds = date.getSeconds().toString().padStart(2, '0');

        return `${hours}:${minutes}:${seconds}`;
    }
</script>
<div>
    <table>
        <thead>
            <tr><th>Time</th><th>Event type</th><th>KeyCode</th><th>Flags</th></tr>
        </thead>
        <tbody>
    {#each event_log as log}
        <tr>
            <td>{formatEpochToHHMMSS(log.timestamp)}</td>
            <td>{log.event_type}</td>
            <td>{getKeyName(log.keycode)}<span class="keycode">({log.keycode})</span></td>
            <td>{flagsToString(log.flags)}</td>
        </tr>
    {/each}
        </tbody>
    </table>
</div>

<style>
    table, th, td {
        border: 1px solid cadetblue;
        padding: 4px;
    }

    table {
        border-collapse: collapse;
    }

    .keycode {
        color: cadetblue;
    }
</style>
