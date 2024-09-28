import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/tauri'

interface Chat {
    id: string
    question: string
    answer: string
    timer: any
}
interface ChatObj {
    [key: string]: Chat[]
}

interface StatusObj {
    [key: string]: {
        status: boolean
    }
}

export const useChatStore = defineStore('chat', () => {
    const ChatObj = ref<ChatObj>({
        // 对话1: [
        //     {
        //         question: '你好1',
        //         answer: '你好啊1'
        //     },
        //     {
        //         question: '你好2',
        //         answer: '你好啊2'
        //     }
        // ],
        // 对话2: [
        //     {
        //         question: '对话2-你好1',
        //         answer: '对话2-你好啊1'
        //     },
        //     {
        //         question: '对话2-你好2',
        //         answer: '对话2-你好啊2'
        //     }
        // ]
    })

    // 对话阻塞状态管理
    const statusObj = ref<StatusObj>({})

    const nowChatName = ref<string>('对话1') // 当前正在用的对话

    const chatRoomList = computed(() => {
        return Object.keys(ChatObj.value)
    })

    const chatList = computed(() => {
        return ChatObj.value[nowChatName.value]
    })

    /**
     * 新增对话
     * @param chatName
     */
    const addChat = (chatName: string) => {
        if (!ChatObj.value[chatName]) {
            ChatObj.value[chatName] = []
            statusObj.value[chatName] = {
                status: false
            }
            nowChatName.value = chatName
            ElMessage({
                message: '新建成功',
                type: 'success'
            })
        } else {
            ElMessage({
                message: '对话不能重名',
                type: 'warning'
            })
        }
        console.log('store新增数据', ChatObj)
    }

    /**
     * 提问
     * @param question
     */
    const sendQuestion = (question: string) => {
        const id = new Date().getTime() + ''
        ChatObj.value[nowChatName.value].push({
            id,
            question,
            answer: '正在思考',
            timer: null
        })
        // 提问时禁用
        statusObj.value[nowChatName.value].status = true
        //调用rust中的方法
        console.log('前端', question, '---', nowChatName.value, id)
        invoke('deal_question', { question: question, name: nowChatName.value, id: id }).then((res: any) => {})

        const index = ChatObj.value[nowChatName.value].findIndex((item) => item.id == id)
        if (index !== -1) {
            // 更新答案
            const nowItem = ChatObj.value[nowChatName.value][index]
            const name = nowChatName.value
            nowItem.timer = setInterval(() => {
                invoke('send_answer', { name: name }).then((res: any) => {
                    if (res) {
                        console.log(res, '---', name, id, ChatObj.value)
                        for (let chatName in ChatObj.value) {
                            const index = ChatObj.value[chatName].findIndex((item) => item.id == id)
                            if (index !== -1) {
                                // 更新答案
                                ChatObj.value[chatName][index].answer = res
                                clearInterval(ChatObj.value[chatName][index].timer)
                                ChatObj.value[chatName][index].timer = null
                                statusObj.value[chatName].status = false
                            }
                        }
                    } else {
                        // console.log('没有答案')
                    }
                })
            }, 1000)
        }
    }

    /**
     * 撤销提问
     */
    const resetQuestion = (resetItem: Chat) => {
        const index = ChatObj.value[nowChatName.value].findIndex((item) => item.id == resetItem.id)
        if (index !== -1) {
            ChatObj.value[nowChatName.value] = ChatObj.value[nowChatName.value].slice(0, index)
        }
        invoke('reset_question', { name: nowChatName.value, id: resetItem.id }).then((res: any) => {})
        console.log('撤销后的数据', ChatObj.value[nowChatName.value])
    }

    const deleteQuestion = (deleteItem: Chat) => {
        const index = ChatObj.value[nowChatName.value].findIndex((item) => item.id == deleteItem.id)
        if (index !== -1) {
            ChatObj.value[nowChatName.value].splice(index, 1)
        }
        console.log('删除后的数据', ChatObj.value[nowChatName.value])
    }

    /**
     * @param chatName 切换对话
     */
    const changeChatRoom = (chatName: string) => {
        nowChatName.value = chatName
    }

    return { chatList, addChat, nowChatName, sendQuestion, resetQuestion, deleteQuestion, chatRoomList, changeChatRoom, statusObj }
})
