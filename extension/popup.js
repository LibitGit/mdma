// Listen for messages from the server page
chrome.runtime.onMessage.addListener(message => {
    console.log(message);
    if (message?.target !== 'Popup') return;
    //console.log(message);

    if ("username" in message && "access_level" in message) {
        try {
            // Close the auth window if it's still open
            console.log(authWindow)
            if (authWindow) {
                authWindow.close();
                authWindow = null;
            }

            if (message.error) {
                throw new Error(message.error);
            }
            //console.log(message);
            showSuccess(message.username, message.access_level);
        } catch (error) {
            showError(`Authentication failed: ${error.message}`);
        }

        return ({
            username,
            access_level
        } = message);
    }

    showLogin();
    ({
        uuid
    } = message);
});

let authWindow = null;
let uuid;

//async function getAuthStatus() {
//  const response = await fetch('http://localhost:3000/api/auth/status', {
//    credentials: 'include'
//  });
//  let status = response.status;
//  let json = await response.json();
//  return { "status": status, "json": json };
//}

const loginBtn = document.getElementById('loginBtn');
const status = document.getElementById('status');
const spinner = document.getElementById('loadingSpinner');
const loginStatus = document.getElementById('loginStatus');

//try {
//  let { status, json } = await getAuthStatus();
//  console.log(status);
//  if (status !== 401 && json.error) throw new Error(json.error);
//  console.log(json, !json.hasOwnProperty("error"));
//  if (!json.hasOwnProperty("error")) return showSuccess(json.sub, json.access);
//} catch(error) {
//  showError(`Authentication failed: ${error.message}`);
//}

function showError(message) {
    status.textContent = message;
    status.className = 'error';
    spinner.style.display = 'none';
    loginStatus.style.display = 'none';
    loginBtn.style.display = 'block';
}

function showSuccess(discord_id, access_level) {
    status.innerHTML = `<strong>Successfully logged in as ${discord_id}!</strong><br>Access level - ${access_level}`;
    status.className = 'success';
    spinner.style.display = 'none';
    loginStatus.style.display = 'none';
    loginBtn.style.display = 'none';
}

function showLogin() {
  spinner.style.display = 'none';
  loginStatus.style.display = 'none';
  loginBtn.style.display = 'block';
}

function showLoading(withoutStatus) {
    spinner.style.display = 'block';
    loginStatus.style.display = withoutStatus ? 'none' : 'block';
    loginBtn.style.display = 'none';
    status.textContent = '';
    status.className = '';
}

showLoading(true);

loginBtn.addEventListener('click', async () => {
    try {
        showLoading();
        chrome.runtime.sendMessage({
            "task": "Login",
            "target": "Background"
        });

        const authUrl = `http://localhost:3000/login/${uuid}`;

        // Open a popup window
        const width = Math.min(800, screen.width * 0.9);
        const height = Math.min(1000, screen.height * 0.9);
        const left = (screen.width - width) / 2;
        const top = (screen.height - height) / 2;

        authWindow = window.open(
            authUrl,
            'DiscordOAuth',
            `width=${width},height=${height},left=${left},top=${top}`
        );
        console.log("Auth window in event:", authWindow?.closed)

        // Start checking if popup is closed
        const popupCheck = setInterval(() => {
            if (authWindow?.closed) {
                clearInterval(popupCheck);
                showError('Authentication window was closed');
            }
        }, 1000);
    } catch (error) {
        showError(`Login failed: ${error.message}`);
    }
});
