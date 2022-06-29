<template>
  <div>
    <Login v-if="!isSignedIn"></Login>
    <Main v-else></Main>
  </div>
</template>

<script>
  import Login from './components/Login.vue';
  import Main from './components/Main.vue';
  export default {
    components: {
      Login,
      Main
    },
    data() {
      return {
        isSignedIn: false
      }
    },
    async mounted() {
      const config = {
        networkId: "testnet",
        keyStore: new window.nearApi.keyStores.BrowserLocalStorageKeyStore(), 
        nodeUrl: "https://rpc.testnet.near.org",
        walletUrl: "https://wallet.testnet.near.org",
        helperUrl: "https://helper.testnet.near.org",
        explorerUrl: "https://explorer.testnet.near.org",
      };

      window.near = await nearApi.connect(config);

      window.wallet = new nearApi.WalletConnection(window.near);

      this.isSignedIn = window.wallet.isSignedIn();
    }
  }
</script>