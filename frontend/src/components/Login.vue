<template>
    <button @click="signIn">Log in with Near</button>
</template>

<script>
export default {
    methods: {
        signIn() {
            window.wallet.requestSignIn(
                "quiztime.arideno.testnet",
            );
        }
    }
}
</script>
