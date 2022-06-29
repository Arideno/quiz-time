<template>
  <div>
    <h4>{{ quiz.question }}</h4>
    <div class="answers">
      <div v-for="(answer, index) in quiz.answers" :key="index">
        <button @click="submitAnswer(index)">{{ answer }}</button>
      </div>
    </div>
  </div>
</template>

<script>
export default {
    props: {
        quiz: {
            type: Object,
            required: true
        }
    },
    methods: {
        async submitAnswer(answerIndex) {
            window.contract.submit_answer({
                hash: this.quiz.hash,
                index: answerIndex
            }).then(response => {
                console.log(response)
            }).catch(error => {
                if (error.kind.ExecutionError == "Smart contract panicked: You can no longer solve this quiz. You are out of tries.") {
                    alert("You are out of tries. Try again later.")
                }
            })
        }
    }
}
</script>