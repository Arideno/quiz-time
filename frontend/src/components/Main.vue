<template>
    <div class="quizzes">
        <div v-for="quiz in quizzes" :key="quiz.hash">
            <Quiz :quiz="quiz"></Quiz>
        </div>
    </div>
</template>

<script>
import Quiz from '@/components/Quiz.vue';
export default {
    components: {
        Quiz
    },
    data() {
        return {
            quizzes: null
        }
    },
    async mounted() {
        const wallet = window.wallet;
        const contract = new nearApi.Contract(
          wallet.account(),
          "quiztime.arideno.testnet",
          {
            viewMethods: ["get_published_quizzes"],
            changeMethods: ["submit_answer"],
            sender: wallet.account(),
          }
        );
        window.contract = contract;

        let response = await contract.get_published_quizzes();
        this.quizzes = response.quizzes;
    }
}
</script>