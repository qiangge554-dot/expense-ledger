<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useLedgerStore } from "@/stores/ledger";
import { addEntry, readFileBase64, importReceipt, listReceipts, deleteReceipt } from "@/lib/commands";
import { pickImages } from "@/lib/dialog";
import { categoriesFor } from "@/constants/categories";
import { STATUS_OPTIONS } from "@/lib/format";
import type {
  LedgerEntry,
  EntryType,
  EntryStatus,
  PaymentMethod,
  ReceiptInfo,
} from "@/types";

const props = defineProps<{ editEntry?: LedgerEntry | null }>();
const emit = defineEmits<{ (e: "saved"): void; (e: "cancel"): void }>();

const store = useLedgerStore();
const isEdit = ref(false);

const ledgerId = ref<number>(store.currentLedgerId);
const date = ref(new Date().toISOString().slice(0, 10));
const type = ref<EntryType>("expense");
const category = ref("");
const amount = ref<number | null>(null);
const party = ref("");
const payment = ref<PaymentMethod>("bank");
const status = ref<EntryStatus>("pending");
const reimbursedAt = ref("");
const note = ref("");
const error = ref("");
const saving = ref(false);

// 常用分类预设（随收入/支出类型切换），点击即填入；输入框仍可自由填写
const presetCategories = computed(() => categoriesFor(type.value));

// 收入不涉及报销；支出默认记为待报销
watch(type, (t) => {
  if (t === "income") {
    status.value = "none";
  } else if (status.value === "none") {
    status.value = "pending";
  }
});
// 标记「已回款」时默认填上今天，省一次点击
watch(status, (s) => {
  if (s === "reimbursed" && !reimbursedAt.value) {
    reimbursedAt.value = new Date().toISOString().slice(0, 10);
  }
  if (s !== "reimbursed") {
    reimbursedAt.value = "";
  }
});

// 停留在录入页时切换顶部账套，记账目标要跟着变（编辑态除外，须保留原账套）
watch(
  () => store.currentLedgerId,
  (id) => {
    if (!isEdit.value) ledgerId.value = id;
  },
);

// 凭证：编辑态用 existing；新增态用 pending
const existing = ref<ReceiptInfo[]>([]);
const pendingPaths = ref<string[]>([]);
const previews = reactive<Record<string, string>>({});

async function loadPreview(path: string) {
  if (previews[path]) return;
  try {
    previews[path] = await readFileBase64(path);
  } catch {
    previews[path] = "";
  }
}

onMounted(async () => {
  if (props.editEntry) {
    isEdit.value = true;
    const e = props.editEntry;
    ledgerId.value = e.ledger_id;
    date.value = e.date;
    type.value = e.type;
    category.value = e.category;
    amount.value = e.amount;
    party.value = e.party ?? "";
    payment.value = (e.payment as PaymentMethod) ?? "bank";
    status.value = (e.status as EntryStatus) ?? "none";
    reimbursedAt.value = e.reimbursed_at ?? "";
    note.value = e.note ?? "";
    existing.value = await listReceipts(e.id);
    existing.value.forEach((r) => loadPreview(r.path));
  }
});

async function addReceipts() {
  const paths = await pickImages(true);
  for (const p of paths) {
    if (isEdit.value && props.editEntry) {
      const saved = await importReceipt(props.editEntry.id, p);
      existing.value.push({ id: Date.now(), path: saved });
      loadPreview(saved);
    } else {
      pendingPaths.value.push(p);
      loadPreview(p);
    }
  }
}

function removeExisting(id: number, path: string) {
  deleteReceipt(id);
  existing.value = existing.value.filter((r) => r.id !== id);
  delete previews[path];
}

function removePending(path: string) {
  pendingPaths.value = pendingPaths.value.filter((p) => p !== path);
  delete previews[path];
}

async function submit(goList: boolean) {
  error.value = "";
  if (amount.value == null || amount.value <= 0) {
    error.value = "请填写大于 0 的金额";
    return;
  }
  if (!category.value.trim()) {
    error.value = "请填写分类";
    return;
  }
  saving.value = true;
  try {
    const payload: Record<string, unknown> = {
      ledger_id: ledgerId.value,
      date: date.value,
      type: type.value,
      category: category.value.trim(),
      amount: amount.value,
      party: party.value.trim() || null,
      payment: payment.value,
      status: status.value,
      reimbursed_at: status.value === "reimbursed" ? reimbursedAt.value || null : null,
      note: note.value.trim() || null,
    };
    if (isEdit.value && props.editEntry) {
      await store.update(props.editEntry.id, payload);
    } else {
      const id = await addEntry(payload);
      for (const p of pendingPaths.value) {
        await importReceipt(id, p);
      }
      await store.refreshAll();
    }
    pendingPaths.value = [];
    emit("saved");
    if (!isEdit.value && !goList) {
      category.value = "";
      amount.value = null;
      party.value = "";
      note.value = "";
    }
  } catch (e) {
    error.value = "保存失败：" + String(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <section class="max-w-2xl">
    <h1 class="text-xl font-medium text-deep mb-4">{{ isEdit ? "编辑记录" : "记一笔" }}</h1>
    <div class="bg-card rounded-xl border border-line shadow-card p-6 space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <label class="text-sm text-ink-400">
          账套
          <select
            v-model="ledgerId"
            class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500"
          >
            <option v-for="l in store.ledgers" :key="l.id" :value="l.id">{{ l.name }}</option>
          </select>
        </label>
        <label class="text-sm text-ink-400">
          日期
          <input
            v-model="date"
            type="date"
            class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500"
          />
        </label>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <label class="text-sm text-ink-400">
          类型
          <select
            v-model="type"
            class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500"
          >
            <option value="expense">支出</option>
            <option value="income">收入</option>
          </select>
        </label>
        <label class="text-sm text-ink-400">
          分类
          <input
            v-model="category"
            list="cat-list"
            placeholder="点选下方常用分类，或直接输入"
            class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500"
          />
          <datalist id="cat-list">
            <option v-for="c in store.categoryList" :key="c" :value="c" />
          </datalist>
          <div class="mt-1.5 flex flex-wrap gap-1.5">
            <button
              v-for="c in presetCategories"
              :key="c"
              type="button"
              @click="category = c"
              :class="[
                'rounded-full border px-2.5 py-0.5 text-xs transition',
                category === c
                  ? 'border-deep bg-deep text-white'
                  : 'border-line text-ink-400 hover:border-deep-500 hover:text-deep',
              ]"
            >
              {{ c }}
            </button>
          </div>
        </label>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <label class="text-sm text-ink-400">
          金额（元）
          <div
            class="mt-1 flex items-center rounded-lg border border-line bg-card-alt focus-within:border-deep-500"
          >
            <span class="pl-3 text-ink-300">¥</span>
            <input
              v-model.number="amount"
              type="number"
              step="0.01"
              placeholder="0.00"
              class="flex-1 bg-transparent px-2 py-2 text-ink outline-none tabular-nums"
            />
          </div>
        </label>
        <label class="text-sm text-ink-400">
          往来方
          <input
            v-model="party"
            placeholder="客户 / 供应商"
            class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500"
          />
        </label>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <label class="block text-sm text-ink-400">
          支付方式
          <select
            v-model="payment"
            class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500"
          >
            <option value="cash">现金</option>
            <option value="bank">银行转账</option>
            <option value="alipay">支付宝</option>
            <option value="wechat">微信</option>
            <option value="card">刷卡</option>
          </select>
        </label>
        <label v-if="type === 'expense'" class="block text-sm text-ink-400">
          报销状态
          <select
            v-model="status"
            class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500"
          >
            <option v-for="s in STATUS_OPTIONS.filter((o) => o.value !== 'none')" :key="s.value" :value="s.value">
              {{ s.label }}
            </option>
          </select>
        </label>
      </div>

      <label v-if="status === 'reimbursed'" class="block text-sm text-ink-400">
        回款日期
        <input
          v-model="reimbursedAt"
          type="date"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500"
        />
      </label>

      <label class="block text-sm text-ink-400">
        备注
        <textarea
          v-model="note"
          rows="2"
          placeholder="可选"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-3 py-2 text-ink outline-none focus:border-deep-500 resize-none"
        ></textarea>
      </label>

      <div>
        <div class="flex items-center justify-between mb-2">
          <span class="text-sm text-ink-400">凭证（本地图片，不上云）</span>
          <button
            type="button"
            @click="addReceipts"
            class="text-xs text-deep border border-deep/40 rounded px-2 py-1 hover:bg-deep/5"
          >
            + 添加凭证
          </button>
        </div>
        <div class="grid grid-cols-4 gap-2">
          <div
            v-for="r in existing"
            :key="'e' + r.id"
            class="relative aspect-square rounded-lg border border-line overflow-hidden group"
          >
            <img v-if="previews[r.path]" :src="previews[r.path]" class="w-full h-full object-cover" />
            <button
              type="button"
              @click="removeExisting(r.id, r.path)"
              class="absolute top-1 right-1 bg-deep/70 text-white rounded-full w-5 h-5 text-xs opacity-0 group-hover:opacity-100"
            >
              ×
            </button>
          </div>
          <div
            v-for="p in pendingPaths"
            :key="'p' + p"
            class="relative aspect-square rounded-lg border border-line overflow-hidden group"
          >
            <img v-if="previews[p]" :src="previews[p]" class="w-full h-full object-cover" />
            <button
              type="button"
              @click="removePending(p)"
              class="absolute top-1 right-1 bg-deep/70 text-white rounded-full w-5 h-5 text-xs opacity-0 group-hover:opacity-100"
            >
              ×
            </button>
          </div>
        </div>
        <p v-if="!existing.length && !pendingPaths.length" class="text-xs text-ink-300">
          暂无凭证，可附发票/收据照片
        </p>
      </div>

      <p v-if="error" class="text-warn-deep text-sm">{{ error }}</p>

      <div class="flex gap-3 pt-1">
        <button
          v-if="!isEdit"
          @click="submit(false)"
          :disabled="saving"
          class="flex-1 rounded-lg bg-deep py-2.5 text-white font-medium hover:bg-deep-600 disabled:opacity-60"
        >
          {{ saving ? "保存中…" : "保存并继续" }}
        </button>
        <button
          v-if="!isEdit"
          @click="submit(true)"
          :disabled="saving"
          class="flex-1 rounded-lg border border-deep text-deep py-2.5 font-medium hover:bg-deep/5 disabled:opacity-60"
        >
          保存并去台账
        </button>
        <button
          v-if="isEdit"
          @click="submit(false)"
          :disabled="saving"
          class="flex-1 rounded-lg bg-deep py-2.5 text-white font-medium hover:bg-deep-600 disabled:opacity-60"
        >
          {{ saving ? "更新中…" : "更新" }}
        </button>
        <button
          v-if="isEdit"
          @click="emit('cancel')"
          class="flex-1 rounded-lg border border-line text-ink-400 py-2.5 hover:bg-card-alt"
        >
          取消
        </button>
      </div>
    </div>
  </section>
</template>
