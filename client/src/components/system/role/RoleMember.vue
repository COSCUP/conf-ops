<template>
  <NFlex
    direction="column"
  >
    <NDataTable
      :columns="columns"
      :loading="loading || removeLoading"
      :data="users"
      :pagination="false"
    />
    <NForm
      class="w-full"
      :model="addUserData"
    >
      <NDynamicInput
        v-model:value="addUserData"
        :on-create="onCreateUser"
      >
        <template #create-button-default>
          {{ t('add_member') }}
        </template>
        <template #action="{ index, remove }">
          <NButton
            ghost
            quaternary
            size="small"
            @click="remove(index)"
          >
            <NIcon>
              <Close />
            </NIcon>
          </NButton>
        </template>
        <template #default="{ index }">
          <NFlex
            class="flex-1"
          >
            <NFlex
              :wrap="false"
            >
              <NFormItem
                ignore-path-change
                :show-label="false"
                :path="`[${index}].name`"
              >
                <NInput
                  v-model:value="addUserData[index].name"
                  :placeholder="t('name')"
                  show-count
                  :maxlength="50"
                />
              </NFormItem>
              <NFormItem
                ignore-path-change
                class="w-[60px]"
                :show-label="false"
                :path="`[${index}].locale`"
              >
                <NSelect
                  v-model:value="addUserData[index].locale"
                  :placeholder="t('locale')"
                  :options="[
                    { label: 'En', value: 'en' },
                    { label: '中', value: 'zh' }
                  ]"
                />
              </NFormItem>
            </NFlex>
            <NDynamicInput
              v-model:value="addUserData[index].emails"
              :on-create="() => ''"
              :min="1"
              class="flex-1"
              item-class="!mb-1"
            >
              <template #default="{ index: sub_index }">
                <NFormItem
                  class="w-full"
                  ignore-path-change
                  :show-label="false"
                  size="small"
                  :path="`[${index}].emails.[${sub_index}]`"
                  :rule="emailRules"
                >
                  <NInput
                    v-model:value="addUserData[index].emails[sub_index]"
                    :placeholder="t('email.name')"
                  />
                </NFormItem>
              </template>
              <template #action="{ index, create, remove }">
                <NButtonGroup class="pl-2">
                  <NButton
                    ghost
                    size="tiny"
                    @click="create(index)"
                  >
                    <NIcon>
                      <Add />
                    </NIcon>
                  </NButton>
                  <NButton
                    ghost
                    size="tiny"
                    @click="remove(index)"
                  >
                    <NIcon>
                      <Subtract />
                    </NIcon>
                  </NButton>
                </NButtonGroup>
              </template>
            </NDynamicInput>
          </NFlex>
        </template>
      </NDynamicInput>
      <NFlex
        v-if="addUserData.length > 0"
        class="w-full"
        justify="space-between"
      >
        <NButton
          ghost
          dashed
          @click="addUserData.push(onCreateUser())"
        >
          {{ t('add_member') }}
          <template #icon>
            <NIcon>
              <Add />
            </NIcon>
          </template>
        </NButton>
        <NButton
          type="primary"
          :loading="addLoading"
          @click="addUsers(addUserData)"
        >
          {{ t('save.title') }}
        </NButton>
      </NFlex>
    </NForm>
  </NFlex>
</template>

<script setup lang="ts">
import { api } from '@/api'
import { User } from '@/api/modules/project'
import { RoleUser } from '@/api/modules/role'
import { useAPI, useActionAPI } from '@/functions/useAPI'
import { DataTableColumns, FormItemRule, NButton, NTime, useDialog } from 'naive-ui'
import { computed, h, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Add, Subtract, Close } from '@vicons/carbon'


const props = defineProps<{
  roleId: string
}>()

const dialog = useDialog()
const { t } = useI18n()

const { data: users, loading, execute: reload } = useAPI(() => api.role.getUsersInManageRole(props.roleId), { default: [] })
const { execute: remove, loading: removeLoading } = useActionAPI((removeUserId: string) => api.role.removeUserInManageRole(props.roleId, removeUserId))
const { execute: addUsers, loading: addLoading } = useActionAPI((addUsers: RoleUser[]) => api.role.addUsersToManageRole(props.roleId, addUsers), () => {
  dialog.success({ content: t('save.success') })
  reload()
  addUserData.value = []
})

const columns = computed<DataTableColumns<User>>(() => [
  {
    key: 'id',
    title: t('id')
  },
  {
    key: 'name',
    title: t('name')
  },
  {
    key: 'locale',
    sorter: true,
    title: t('locale')
  },
  {
    key: 'created_at',
    title: t('created_at'),
    render: (row) => h(NTime, { value: row.created_at }),
    sorter: (a, b) => a.created_at.getTime() - b.created_at.getTime()
  },
  {
    key: 'actions',
    title: t('actions'),
    render(row) {
      return h(
        NButton,
        {
          onClick() {
            dialog.warning({
              title: t('remove'),
              content: t('remove_confirm', { name: row.name }),
              positiveText: t('ok'),
              negativeText: t('cancel'),
              onPositiveClick() {
                remove(row.id)
                  .then(() => {
                    dialog.success({ content: t('remove_success') })
                    reload()
                  })
              }
            })
          }
        },
        { default: () => t('remove') }
      )
    }
  }
])

const addUserData = ref<RoleUser[]>([])

const onCreateUser = () => ({ name: '', locale: 'zh' as const, emails: [''] })

const emailRules = computed(() => [
  { required: true, message: t('email.required'), trigger: 'blur' },
  { type: 'email', message: t('email.invalid'), trigger: ['input', 'blur'] }
] as FormItemRule[])


</script>

<i18n lang="json">
{
  "en": {
    "id": "User ID",
    "name": "User Name",
    "locale": "Locale",
    "created_at": "Created At",
    "actions": "Actions",
    "email": {
      "name": "Email",
      "required": "Email is required",
      "invalid": "Invalid email"
    },
    "remove": "Remove",
    "remove_confirm": "Are you sure to remove user {name} from this role?",
    "remove_success": "User removed",
    "add_member": "Add Member"
  },
  "zh": {
    "id": "用戶 ID",
    "name": "用戶名",
    "locale": "語言",
    "created_at": "創建時間",
    "actions": "操作",
    "email": {
      "name": "Email",
      "required": "Email 是必需的",
      "invalid": "無效的 Email"
    },
    "remove": "移除",
    "remove_confirm": "確定要將用戶 {name} 從該角色中移除嗎？",
    "remove_success": "用戶已移除",
    "add_member": "新增成員"
  }
}
</i18n>
