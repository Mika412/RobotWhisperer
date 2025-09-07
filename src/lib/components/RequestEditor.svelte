<script lang="ts">
	import {
		Save,
		Edit2,
		Zap,
		Trash2,
		Bot,
		ArrowLeftRight,
	} from "@lucide/svelte";
	import {
		activeItem,
		saveActiveItem,
		updateActiveItemField,
	} from "$lib/stores/workspaceStore";
	import { deleteRequest } from "$lib/stores/requestStore";
	import type { RosRequest } from "$lib/db";

	const specifics = {
		topic: {
			placeholder: "topic name (e.g., /scan)",
			actionText: "Subscribe",
			Icon: Zap,
		},
		service: {
			placeholder: "service name (e.g., /add_two_ints)",
			actionText: "Call",
			Icon: ArrowLeftRight,
		},
		action: {
			placeholder: "action name (e.g., /fibonacci)",
			actionText: "Send",
			Icon: Bot,
		},
	};

	function getBadgeClass(tag?: string) {
		switch (tag) {
			case "topic":
				return "badge-topic focus:ring-[var(--color-badge-topic-text)]/50";
			case "service":
				return "badge-service focus:ring-[var(--color-badge-service-text)]/50";
			case "action":
				return "badge-action focus:ring-[var(--color-badge-action-text)]/50";
			default:
				return "bg-gray-500/10 text-gray-400 border-gray-500/30 focus:ring-gray-400/50";
		}
	}
</script>

{#if $activeItem}
	{@const spec = specifics[$activeItem.data.type]}
	<div class="flex flex-col h-full bg-bg-main">
		<div class="p-4 border-b border-border flex-shrink-0 space-y-4">
			<div class="flex items-center gap-3 group">
				<input
					type="text"
					value={$activeItem.data.name}
					oninput={(e) =>
						updateActiveItemField("name", e.currentTarget.value)}
					class="text-xl font-bold bg-transparent focus:outline-none focus:bg-bg-input rounded p-1 -m-1 border border-transparent focus:border-accent hover:border-border transition-colors"
				/>
				<Edit2
					size={16}
					class="text-text-disabled opacity-0 group-hover:opacity-100 transition-opacity"
				/>
				<div
					class="ml-auto flex items-center gap-1 bg-bg-input border border-border rounded-lg p-0.5"
				>
					<button
						onclick={() => deleteRequest($activeItem.id)}
						class="p-2 rounded-md hover:bg-red-500/20 text-text-dimmer hover:text-red-400 transition-colors"
					>
						<Trash2 size={16} />
					</button>
					<button
						onclick={saveActiveItem}
						disabled={!$activeItem.isDirty}
						class="flex items-center gap-2 bg-accent text-white px-3 py-1.5 rounded-md hover:bg-accent-dark disabled:bg-bg-disabled disabled:text-text-disabled disabled:cursor-not-allowed text-sm font-semibold transition-colors"
					>
						<Save size={16} /> Save
					</button>
				</div>
			</div>

			<div
				class="flex items-center gap-2 bg-bg-input border border-border rounded-lg p-1"
			>
				<select
					value={$activeItem.data.type}
					onchange={(e) =>
						updateActiveItemField(
							"type",
							e.currentTarget.value as RosRequest["type"],
						)}
					class="font-bold border rounded-md px-3 py-2.5 focus:outline-none focus:ring-2 transition-colors uppercase text-sm {getBadgeClass(
						$activeItem.data.type,
					)} disabled:opacity-50 disabled:cursor-not-allowed"
				>
					<option value="topic">Topic</option>
					<option value="service">Service</option>
					<option value="action">Action</option>
				</select>
				<div class="flex-grow">
					<input
						type="text"
						placeholder={spec.placeholder}
						value={$activeItem.data.target}
						oninput={(e) =>
							updateActiveItemField("target", e.currentTarget.value)}
						class="w-full bg-transparent px-3 py-2.5 focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed"
					/>
				</div>
				<button
					disabled={!$activeItem.data.target ||
						$activeItem.data.target.trim() === ""}
					class="text-white px-4 py-2 rounded-md flex items-center justify-center gap-2 font-semibold text-sm transition-colors mr-1 w-36 bg-green-500/80 hover:bg-green-500 disabled:bg-bg-disabled disabled:text-text-disabled disabled:cursor-not-allowed"
				>
					<svelte:component this={spec.Icon} size={16} />
					{spec.actionText}
				</button>
			</div>
			<div>
				<label
					class="text-xs font-semibold text-text-dimmer block mb-1.5 ml-1"
					>Message Type</label
				>
				<div class="w-full">
					<input
						type="text"
						placeholder="e.g., sensor_msgs/LaserScan"
						value={$activeItem.data.interface}
						oninput={(e) =>
							updateActiveItemField("interface", e.currentTarget.value)}
						class="w-full bg-bg-input border border-border rounded-lg px-3 py-2.5 focus:outline-none focus:ring-2 focus:ring-accent disabled:opacity-50 disabled:cursor-not-allowed"
					/>
				</div>
			</div>
		</div>

		<div class="p-4 overflow-y-auto flex-grow">
			<div class="space-y-3">
				<h3 class="text-sm font-semibold text-text-dimmer mb-2">
					Message Fields
				</h3>
				<p class="text-sm text-text-disabled">
					Select a valid message type to see its fields.
				</p>
			</div>
		</div>
	</div>
{/if}

