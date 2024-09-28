<script setup lang="ts">
import { ref } from 'vue'
import Home from '../Home/index.vue'
import { useChatStore } from '@/store'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
const ChatStore = useChatStore()

const question = ref('')
const showAddChatRoomDialog = ref<boolean>(false)

interface Chat {
    id: string
    question: string
    answer: string
    timer: any
}

const sendQuestion = () => {
    if (question.value) {
        ChatStore.sendQuestion(question.value)
        question.value = ''
    } else {
        ElMessage({
            message: '请输入您的问题',
            type: 'warning'
        })
    }
}

const showAddDialog = () => {
    showAddChatRoomDialog.value = true
}

const closeAddDialog = () => {
    showAddChatRoomDialog.value = false
}

const resetQuestion = (item: Chat) => {
    if (ChatStore.statusObj[ChatStore.nowChatName].status) {
        ElMessage({
            message: '当前有未完成的提问，请先完成',
            type: 'warning'
        })
        return
    }
    ElMessageBox.confirm('确定撤销这条提问吗?撤销操作无法回退！', '提示', {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
    })
        .then(() => {
            question.value = item.question
            ChatStore.resetQuestion(item)
            ElMessage({
                type: 'success',
                message: '撤销成功'
            })
        })
        .catch(() => {})
}

// const deleteQuestion = (item: Chat) => {
//     ElMessageBox.confirm('确定删除这条提问吗?删除操作无法回退！', '提示', {
//         confirmButtonText: '确定',
//         cancelButtonText: '取消',
//         type: 'warning'
//     })
//         .then(() => {
//             ChatStore.deleteQuestion(item)
//             ElMessage({
//                 type: 'success',
//                 message: '删除成功'
//             })
//         })
// }

const copyFun = (content: string) => {
    navigator.clipboard.writeText(content)
    ElMessage({
        message: '复制成功',
        type: 'success'
    })
}
</script>

<template>
    <div class="chat-page">
        <div class="chat-room-list">
            <div class="chat-room-title">聊天室列表</div>
            <el-button color="#615ced" class="add-chat-room-btn" round :icon="Plus" @click="showAddDialog"
                >新建聊天室</el-button
            >
            <div
                :class="`${item === ChatStore.nowChatName ? 'chat-room-item-active' : 'chat-room-item'}`"
                @click="ChatStore.changeChatRoom(item)"
                v-for="item in ChatStore.chatRoomList"
                :key="item"
            >
                {{ item }}
            </div>
        </div>
        <div class="chat-room-content">
            <div class="chat-name">{{ ChatStore.nowChatName }}</div>

            <div class="chat-box">
                <div v-for="(item, index) in ChatStore.chatList" :key="index" class="chat-item">
                    <div class="question">
                        <span>{{ item.question }}</span>
                        <div>
                            <img class="operate-btn" @click="copyFun(item.question)" src="@/assets/copy.svg" alt="" />
                            <img class="operate-btn" @click="resetQuestion(item)" src="@/assets/reset.svg" alt="" />
                            <!-- <img class="operate-btn" @click="deleteQuestion(item)" src="@/assets/delete.svg" alt="" /> -->
                        </div>
                    </div>
                    <div class="answer">
                        <img class="answer-avatar" src="@/assets/avatar.png" alt="" />
                        <div class="answer-content">
                            <span>{{ item.answer }}</span>
                            <img class="operate-btn" @click="copyFun(item.answer)" src="@/assets/copy.svg" alt="" />
                        </div>
                    </div>
                </div>
            </div>

            <div class="question-box">
                <el-input
                    v-model="question"
                    @keyup.enter="sendQuestion"
                    class="question-input"
                    placeholder="请输入您的问题"
                ></el-input>
                <el-button
                    class="submit-btn"
                    type="primary"
                    :disabled="ChatStore.statusObj[ChatStore.nowChatName].status"
                    @click="sendQuestion"
                    >发送</el-button
                >
            </div>
        </div>

        <el-dialog v-model="showAddChatRoomDialog" destroy-on-close title="新建聊天室" width="500">
            <Home @submit="closeAddDialog"></Home>
        </el-dialog>
    </div>
</template>

<style scoped lang="less">
.chat-page {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    // justify-content: center;

    .chat-room-list {
        width: 200px;
        height: 100%;
        padding: 0 5px;
        .chat-room-title {
            font-size: 18px;
            margin: 10px 0 0 10px;
        }
        .add-chat-room-btn {
            width: 100%;
            margin: 10px 0 0 0;
            cursor: pointer;
        }
        .chat-room-item,
        .chat-room-item-active {
            height: 36px;
            border-radius: 12px;
            background-color: white;
            display: flex;
            align-items: center;
            padding: 2px 10px;
            cursor: pointer;
            margin: 10px 0;
        }
        .chat-room-item-active,
        .chat-room-item:hover {
            color: white;
            background-color: #615ced;
        }
    }
    .chat-room-content {
        width: 100%;
        height: 100%;
        display: flex;
        flex-direction: column;
        align-items: center;
        flex: 1;
    }

    .chat-name {
        font-size: 24px;
    }

    .chat-box {
        flex: 1;
        width: 100%;
        background-color: #f7f8fc;
        padding: 0 20px;
        box-sizing: border-box;
        overflow-y: auto;
        overflow-x: hidden;
        .chat-item {
            display: flex;
            flex-direction: column;
            font-size: 18px;
            margin: 16px 0;
            .question {
                align-self: end;
                display: flex;
                flex-direction: column;
                align-items: flex-end;
                background-color: #e0dfff;
                border-radius: 12px;
                line-height: 24px;
                max-width: 100%;
                padding: 12px 16px;
            }
            .answer {
                align-self: start;
                display: flex;
                margin-top: 16px;
                .answer-avatar {
                    width: 36px;
                    height: 36px;
                }
                .answer-content {
                    background-color: white;
                    border: 1px solid transparent;
                    border-radius: 12px;
                    padding: 16px;
                    margin-left: 10px;
                    display: flex;
                    flex-direction: column;
                    align-items: flex-end;
                }
            }
            .operate-btn {
                    width: 20px;
                    height: 20px;
                    border-radius: 6px;
                    margin: 5px 5px 0 5px;
                    cursor: pointer;
                }
        }
    }
    .question-box {
        display: flex;
        align-items: center;
        width: 100%;
        height: 60px;
        padding: 10px 10px;
        box-sizing: border-box;
        .question-input {
            width: 90%;
        }
        .submit-btn {
            margin-left: 10px;
        }
    }
}
</style>
