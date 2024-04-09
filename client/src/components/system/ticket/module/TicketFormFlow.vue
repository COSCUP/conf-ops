<template>
  <NForm
    ref="formRef"
    :model="formData"
    :rules="rules"
    @submit.prevent="execute"
  >
    <NFormItem
      v-for="field, index in schema.fields"
      :key="field.id"
      ref="formItemRefs"
      class="mb-6"
      :path="field.key"
      :label="field[`name_${locale}`]"
      label-placement="top"
    >
      <NFlex
        class="w-full"
        vertical
      >
        <template v-if="field[`description_${locale}`]">
          <NText
            class="ml-[2px] text-sm text-gray whitespace-pre-line"
          >
            {{ field[`description_${locale}`] }}
          </NText>
        </template>
        <template v-if="field.define.type === 'SingleLineText'">
          <NInput
            v-model:value="formData[field.key]"
            show-count
            :maxlength="field.define.max_texts"
            :disabled="!field.editable || isReview"
          />
        </template>
        <template v-else-if="field.define.type === 'MultiLineText'">
          <NInput
            v-model:value="formData[field.key]"
            type="textarea"
            show-count
            :maxlength="field.define.max_texts"
            :disabled="!field.editable || isReview"
          />
        </template>
        <template v-else-if="field.define.type === 'SingleChoice'">
          <NSelect
            v-model:value="formData[field.key]"
            :options="convertOptions(field.define.options)"
            :disabled="!field.editable || isReview"
          />
        </template>
        <template v-else-if="field.define.type === 'MultipleChoice'">
          <template v-if="field.define.is_checkbox">
            <NCheckboxGroup
              v-model:value="formData[field.key]"
              :disabled="!field.editable || isReview"
            >
              <NFlex>
                <NCheckbox
                  v-for="option in field.define.options"
                  :key="option.value"
                  :label="option.text"
                  :value="option.value"
                />
              </NFlex>
            </NCheckboxGroup>
          </template>
          <template v-else>
            <NSelect
              v-model:value="formData[field.key]"
              multiple
              :options="convertOptions(field.define.options)"
              :disabled="!field.editable || isReview"
            />
          </template>
        </template>
        <template v-else-if="field.define.type === 'Bool'">
          <NCheckbox
            v-model:checked="formData[field.key]"
            :disabled="!field.editable || isReview"
          />
        </template>
        <template v-else-if="field.define.type === 'Image'">
          <NUpload
            v-model:file-list="formData[field.key]"
            :action="`/api/project/ticket/schemas/${ticketSchemaId}/form/${schema.form.id}/field/${field.id}/upload`"
            with-credentials
            response-type="json"
            :max="1"
            list-type="image-card"
            trigger-class="!w-96px !h-96px"
            :accept="field.define.mimes.join(',')"
            :disabled="!field.editable || isReview"
            @finish="handleUploadFinished"
            @error="handleUploadError"
            @before-upload="handleBeforeUpload(field.key)($event)"
            @update-file-list="formItemRefs[index]?.validate"
          >
            <NUploadDragger class="flex flex-col justify-center items-center">
              <NIcon
                class="mb-3"
                :size="48"
                :depth="3"
              >
                <CloudUpload />
              </NIcon>
              <NText class="text-sm">
                {{ t('upload') }}
              </NText>
            </NUploadDragger>
          </NUpload>
        </template>
        <template v-else-if="field.define.type === 'File'">
          <NUpload
            v-model:file-list="formData[field.key]"
            :action="`/api/project/ticket/schemas/${ticketSchemaId}/form/${schema.form.id}/field/${field.id}/upload`"
            with-credentials
            response-type="json"
            :max="1"
            :accept="field.define.mimes.join(',')"
            :disabled="!field.editable || isReview"
            @finish="handleUploadFinished"
            @error="handleUploadError"
            @before-upload="handleBeforeUpload(field.key)($event)"
            @update-file-list="formItemRefs[index]?.validate"
          >
            <NUploadDragger class="flex flex-col justify-center items-center">
              <NIcon
                class="mb-3"
                :size="48"
                :depth="3"
              >
                <CloudUpload />
              </NIcon>
              <NText class="text-sm">
                {{ t('upload') }}
              </NText>
            </NUploadDragger>
          </NUpload>
        </template>
      </NFlex>
    </NFormItem>
    <NButton
      v-if="!isReview"
      type="primary"
      :loading="loading"
      attr-type="submit"
    >
      {{ t('save.title') }}
    </NButton>
  </NForm>
</template>

<script setup lang="ts">
import { TicketFormSchema, TicketFormProcessFlow, FormFieldDefault, FormFieldDefine, TicketFormValue } from '@/api/modules/ticket/types'
import { computed, ref } from 'vue'
import { CloudUpload } from '@vicons/carbon'
import { useI18n } from 'vue-i18n'
import { getAPIErrorMessage, useFormAPI } from '@/functions/useAPI'
import { api } from '@/api'
import { FormInst, FormItemInst, FormItemRule, FormRules, NFlex, UploadFileInfo, useDialog, useMessage } from 'naive-ui'
import { getImageSize } from '@/utils/image'
import { useLocale } from '@/i18n'

const props = defineProps<{
  id: number
  ticketSchemaId: number
  schema: TicketFormSchema,
  formValue: TicketFormValue | null,
  isReview?: boolean
}>()

const emit = defineEmits<{
  refresh: []
}>()

const dialog = useDialog()
const message = useMessage()
const { t } = useI18n()
const { locale } = useLocale()

const getFieldDefaultValue = <T>(field?: FormFieldDefault<T>) => {
  if (!field) {
    return undefined
  }
  if (field.type === 'Static') {
    return field.content
  }
  return field.content.value
}

const getDefaultFormData = () => {
  const data: TicketFormProcessFlow = {
    type: 'Form'
  }
  for (const field of props.schema.fields) {
    if (field.define.type === 'IfEqual' || field.define.type === 'IfEnd') continue
    const value = props.formValue?.value?.[field.key] ?? getFieldDefaultValue<unknown>(field.define.default)
    if (field.define.type === 'Image' || field.define.type === 'File') {
      if (!value) {
        data[field.key] = []
        continue
      }
      formFiles.set(value as string, value as string)
      data[field.key] = [
        {
          id: value,
          name: '',
          status: 'finished',
          url: `${window.location.protocol}//${window.location.host}/api/project/ticket/schemas/${props.id}/form/${props.schema.form.id}/field/${field.id}/${value}`
        }
      ] as UploadFileInfo[]
      continue
    }
    data[field.key] = value
  }
  return data
}

const formRef = ref<FormInst | null>(null)
const formItemRefs = ref<FormItemInst[]>([])
const formFiles = new Map<string, string>()
const formData = ref<TicketFormProcessFlow>(getDefaultFormData())

const convertOptions = (options: Array<{ text: string, value: unknown }>) =>
  options.map(option => ({
    label: option.text,
    value: option.value as any
  }))

const handleBeforeUpload = (key: string) => async ({ file }: { file: UploadFileInfo }) => {
  const fieldRules = rules.value[key] as FormItemRule[]
  for (const rule of fieldRules) {
    if (rule.required) continue

    const validate = rule.validator as (_rule: FormItemRule, value: UploadFileInfo[]) => boolean | Promise<void>
    if (!validate) continue
    const validateResult = validate(rule, [file])
    if (validateResult instanceof Promise) {
      try {
        await validateResult
      } catch (err) {
        message.error(rule.message as string)
        return false
      }
    }
    if (!validateResult) {
      message.error(rule.message as string)
      return false
    }
  }
  return true
}

const handleUploadError = ({ event }: { file: UploadFileInfo, event?: ProgressEvent }) => {
  const error = (event?.target as XMLHttpRequest).response as { message: string }
  message.error(error.message)
}

const handleUploadFinished = ({ file, event }: { file: UploadFileInfo, event?: ProgressEvent }) => {
  const response = (event?.target as XMLHttpRequest).response as { id: string } | undefined
  if (response) {
    formFiles.set(file.id, `${response.id}.${file.name.slice().split('.').pop()}`)
  }
  return file
}

const { execute, loading } = useFormAPI(
  () => {
    const data = { ...formData.value }
    for (const field of props.schema.fields) {
      const type = field.define.type
      if (!field.editable) {
        delete data[field.key]
        continue
      }
      if (type === 'Image' || type === 'File') {
        const file = formData.value?.[field.key]?.[0] as UploadFileInfo
        if (file) {
          data[field.key] = formFiles.get(file.id)
        } else {
          delete data[field.key]
        }
      }
    }
    return api.ticket.process(props.id, data)
  },
  () => formRef.value,
  () => {
    dialog.success({ content: t('save.success') })
    emit('refresh')
  },
  {
    failure: (error) => {
      let content = getAPIErrorMessage(error)
      if (Object.keys(error.fields).length > 0) {
        content = Object.keys(error.fields).map((key) => `${props.schema.fields.find((f) => f.key === key)?.[`name_${locale.value}`] ?? key}: ${error.fields[key]}`).join('\n')
      }

      dialog.error({
        title: t('error'),
        content,
        class: 'whitespace-pre-line',
        positiveText: t('ok')
      })
    }
  }
)

const rules = computed(() => {
  const getType = (field: FormFieldDefine): FormItemRule['type'] => {
    switch (field.type) {
      case 'SingleLineText':
      case 'MultiLineText':
        return 'string'
      case 'Image':
      case 'File':
      case 'MultipleChoice':
        return 'array'
      case 'Bool':
        return 'boolean'
      default:
        return undefined
    }
  }

  const result: FormRules = {}
  for (const field of props.schema.fields) {
    const rules: FormItemRule[] = []
    if (field.required) {
      rules.push({
        type: getType(field.define),
        required: true,
        message: t('rule.required'),
        trigger: 'blur'
      })
    }
    if (field.define.type === 'SingleLineText' || field.define.type === 'MultiLineText') {
      if (field.define.max_texts) {
        rules.push({
          type: 'string',
          max: field.define.max_texts,
          message: t('rule.max_texts', [field.define.max_texts]),
          trigger: ['input', 'blur']
        })
      }
      if (field.define.type === 'MultiLineText' && field.define.max_lines) {
        const max_lines = field.define.max_lines
        rules.push({
          validator: (_rule, value) => (value?.split('\n').length ?? 0) <= max_lines,
          message: t('rule.max_lines', [max_lines]),
          trigger: ['input', 'blur']
        })
      }
      if (field.define.type === 'SingleLineText' && field.define.text_type) {
        rules.push({
          type: field.define.text_type,
          message: t(`rule.${field.define.text_type}`),
          trigger: ['input', 'blur']
        })
      }
    }

    if (field.define.type === 'MultipleChoice') {
      rules.push({
        type: 'array',
        max: field.define.max_options,
        message: t('rule.max_options', [field.define.max_options]),
        trigger: ['input', 'blur']
      })
    }

    if (field.define.type === 'Image') {
      const { mimes, min_width, max_width, min_height, max_height, max_size } = field.define
      rules.push({
        validator: (_rule, value: UploadFileInfo[]) => {
          if (value.length === 0) {
            return true
          }
          if (formFiles.has(value[0].id)) {
            return true
          }
          return mimes.includes(value[0]?.type ?? '')
        },
        message: t('rule.image_mimes', [mimes.join(', ')]),
        trigger: 'change',
      })
      rules.push({
        validator: async (_rule, value: UploadFileInfo[]) => {
          if (value.length === 0) {
            return
          }
          if (formFiles.has(value[0].id)) {
            return
          }
          const file = value[0]?.file
          if (!file) {
            throw Error()
          }
          const { width, height } = await getImageSize(file)
          if (min_width && width < min_width) {
            throw Error()
          }
          if (max_width && width > max_width) {
            throw Error()
          }
          if (min_height && height < min_height) {
            throw Error()
          }
          if (max_height && height > max_height) {
            throw Error()
          }
        },
        message: t('rule.size', [min_width, max_width, min_height, max_height])
      })
      rules.push({
        validator: (_rule, value: UploadFileInfo[]) => {
          if (value.length === 0) {
            return true
          }
          if (formFiles.has(value[0].id)) {
            return true
          }
          return (value[0]?.file?.size ?? 0) <= max_size
        },
        message: t('rule.max_size', [max_size]),
        trigger: 'change',
      })
    }

    if (field.define.type === 'File') {
      const { mimes, max_size } = field.define
      rules.push({
        validator: (_rule, value: UploadFileInfo[]) => {
          if (value.length === 0) {
            return true
          }
          if (formFiles.has(value[0].id)) {
            return true
          }
          return mimes.includes(value[0]?.type ?? '')
        },
        message: t('rule.file_mimes', [mimes.join(', ')]),
        trigger: 'change',
      })
      rules.push({
        validator: (_rule, value: UploadFileInfo[]) => {
          if (value.length === 0) {
            return true
          }
          if (formFiles.has(value[0].id)) {
            return true
          }
          return (value[0]?.file?.size ?? 0) <= max_size
        },
        message: t('rule.max_size', [max_size]),
        trigger: 'change',
      })
    }
    result[field.key] = rules
  }
  return result
})
</script>

<i18n lang="json">
{
  "en": {
    "upload": "Upload",
    "rule": {
      "required": "This field is required",
      "max_texts": "The maximum number of characters is {0}",
      "max_lines": "The maximum number of lines is {0}",
      "max_options": "The maximum number of options is {0}",
      "image_mimes": "The image must be in {0} format",
      "size": "The image size must be between {0}x{2} and {1}x{3}",
      "file_mimes": "The file must be in {0} format",
      "max_size": "The file size must be less than {0} bytes",
      "string": "Please enter a string",
      "email": "Please enter a valid email address",
      "url": "Please enter a valid URL"
    }
  },
  "zh": {
    "upload": "上傳",
    "rule": {
      "required": "此欄位為必填",
      "max_texts": "最多輸入 {0} 個字元",
      "max_lines": "最多輸入 {0} 行",
      "max_options": "最多選擇 {0} 項",
      "image_mimes": "圖片必須為 {0} 格式",
      "size": "圖片尺寸必須介於 {0}x{2} 與 {1}x{3} 之間",
      "file_mimes": "檔案必須為 {0} 格式",
      "max_size": "檔案大小必須小於 {0} bytes",
      "string": "請輸入字串",
      "email": "請輸入正確的電子郵件格式",
      "url": "請輸入正確的網址格式"
    }
  }
}
</i18n>
