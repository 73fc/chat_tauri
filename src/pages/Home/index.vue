<script setup lang="ts">
import { ElMessage } from 'element-plus'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { useChatStore } from '@/store'
const ChatStore = useChatStore()
const router = useRouter()
const emit = defineEmits(['submit'])

const chatName = ref('')
const addChat = () => {
    if (chatName.value) {
        ChatStore.addChat(chatName.value)
        router.push('./Chat')
        emit('submit')
    } else {
        ElMessage({
            message: '请输入对话名字',
            type: 'warning'
        })
    }
}
</script>

<template>
    <div class="home-page">
        <div class="flex2 home-page">
            <h1>新建对话</h1>
            <el-input v-model="chatName" class="add-chat-input" placeholder="请输入对话名字"></el-input>
            <el-button class="add-chat-btn" type="primary" @click="addChat">新建</el-button>
        </div>
        <div class="flex1"></div>
    </div>
</template>

<style scoped lang="less">
.home-page {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    box-sizing: border-box;
    .add-chat-input {
        width: 50%;
    }
    .add-chat-btn {
        margin-top: 20px;
    }
    .flex1 {
        flex: 1;
    }
    .flex2 {
        flex: 2
    }
}
</style>
